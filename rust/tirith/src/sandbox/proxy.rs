//! The daemon's HTTP proxy for sandboxed processes: `CONNECT host:port` and plain-HTTP absolute-form requests.
//! It authenticates the caller, refuses blocked destinations (by name, then by the addresses they resolve to),
//! records each new destination of each caller, and pipes the bytes.

use super::{blocked, owner, proxy_port, token};
use crate::app::{now_iso, now_s, Shared};
use base64::Engine;
use serde_json::{json, Value};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    net::SocketAddr,
    sync::{LazyLock, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[derive(Default)]
struct Dest {
    count: u64,
    refused: u64,
    last: String,
    lanes: BTreeSet<String>,
}

/// Destinations by host, the (who, host:port) pairs already recorded, refusals recorded in the last minute.
static DESTS: LazyLock<Mutex<HashMap<String, Dest>>> = LazyLock::new(Default::default);
static SEEN: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);
static REFUSED: LazyLock<Mutex<HashMap<String, u64>>> = LazyLock::new(Default::default);
static STATE: Mutex<String> = Mutex::new(String::new());

pub fn state() -> String {
    STATE.lock().unwrap().clone()
}

/// The destinations seen so far from the `net` / `net-blocked` events, so the page keeps its history.
pub fn seed(app: &Shared) {
    let sql = "SELECT summary, lane, kind, COUNT(*), MAX(at) FROM events WHERE kind IN ('net', 'net-blocked') GROUP BY summary, lane, kind";
    let rows = app.query(sql, [], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, u64>(3)?, r.get::<_, String>(4)?)));
    let mut dests = DESTS.lock().unwrap();
    for (dest, lane, kind, n, at) in rows {
        let host = dest.rsplit_once(':').map(|(h, _)| h).unwrap_or(&dest).to_string();
        let d = dests.entry(host).or_default();
        let counter = if kind == "net" { &mut d.count } else { &mut d.refused };
        *counter += n;
        d.last = d.last.clone().max(at);
        d.lanes.insert(lane);
    }
}

/// The destinations, newest first: host, connections, refusals, callers, last seen and the rule blocking it now.
pub fn destinations() -> Value {
    let dests = DESTS.lock().unwrap();
    let mut out: Vec<Value> = dests.iter().map(|(h, d)| json!({"host": h, "count": d.count, "refused": d.refused, "lanes": d.lanes, "last": d.last, "rule": blocked(h)})).collect();
    out.sort_by(|a, b| b["last"].as_str().cmp(&a["last"].as_str()));
    out.truncate(300);
    json!(out)
}

/// Listens on 127.0.0.1:<proxy_port> for the daemon's lifetime (retrying while the port is taken).
pub async fn serve(app: Shared) {
    let port = proxy_port(&app);
    let mut told = false;
    loop {
        match TcpListener::bind(("127.0.0.1", port)).await {
            Ok(listener) => {
                *STATE.lock().unwrap() = "listening".into();
                loop {
                    let Ok((conn, _)) = listener.accept().await else { continue };
                    tokio::spawn(handle(app.clone(), conn));
                }
            }
            Err(e) => {
                *STATE.lock().unwrap() = format!("cannot listen on 127.0.0.1:{port}: {e}");
                if !std::mem::replace(&mut told, true) {
                    app.event("", "", "error", "proxy", &state(), "");
                }
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

async fn answer(c: &mut TcpStream, status: &str, extra: &str, body: &str) -> Result<(), String> {
    let text = format!("HTTP/1.1 {status}\r\n{extra}Content-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    c.write_all(text.as_bytes()).await.map_err(|e| e.to_string())
}

/// The caller of a request: the `who` of a valid `Proxy-Authorization: Basic who:token`.
fn caller(headers: &[&str]) -> Option<String> {
    let value = headers.iter().find_map(|h| h.split_once(':').filter(|(k, _)| k.trim().eq_ignore_ascii_case("proxy-authorization")))?.1.trim();
    let (_, data) = value.split_once(' ').filter(|(scheme, _)| scheme.eq_ignore_ascii_case("basic"))?;
    let raw = String::from_utf8(base64::engine::general_purpose::STANDARD.decode(data.trim()).ok()?).ok()?;
    let (user, pass) = raw.rsplit_once(':')?;
    let who = user.replace("%3A", ":").replace("%3a", ":").replace('~', ":");
    (pass == token(&who)).then_some(who)
}

/// `host[:port]` with an optional `[v6]` host.
fn host_port(text: &str, default: u16) -> Option<(String, u16)> {
    let (host, port) = match text.strip_prefix('[') {
        Some(v6) => {
            let (h, rest) = v6.split_once(']')?;
            (h, rest.strip_prefix(':'))
        }
        None => text.rsplit_once(':').map_or((text, None), |(h, p)| (h, Some(p))),
    };
    let port = port.map_or(Some(default), |p| p.parse().ok())?;
    (!host.is_empty()).then(|| (host.trim_end_matches('.').to_lowercase(), port))
}

fn record(app: &Shared, who: &str, host: &str, port: u16, rule: Option<&str>) {
    let (lane, project) = owner(app, who);
    {
        let mut dests = DESTS.lock().unwrap();
        let d = dests.entry(host.to_string()).or_default();
        let counter = if rule.is_some() { &mut d.refused } else { &mut d.count };
        *counter += 1;
        d.last = now_iso();
        d.lanes.insert(lane.clone());
    }
    let key = format!("{who}|{host}:{port}");
    let new = match rule {
        Some(_) => REFUSED.lock().unwrap().insert(key, now_s()).is_none_or(|t| now_s() - t >= 60),
        None => SEEN.lock().unwrap().insert(key),
    };
    if new {
        let kind = if rule.is_some() { "net-blocked" } else { "net" };
        app.event(&lane, &project, kind, "proxy", &format!("{host}:{port}"), "");
    }
}

async fn handle(app: Shared, mut c: TcpStream) -> Result<(), String> {
    let mut buf = Vec::with_capacity(2048);
    let end = loop {
        let mut chunk = [0u8; 4096];
        let n = tokio::time::timeout(Duration::from_secs(30), c.read(&mut chunk)).await.map_err(|_| "slow client")?.map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        if buf.len() > 65536 {
            return answer(&mut c, "431 Request Header Fields Too Large", "", "").await;
        }
    };
    let head = String::from_utf8_lossy(&buf[..end]).to_string();
    let mut lines = head.split("\r\n");
    let mut first = lines.next().unwrap_or("").split(' ');
    let (method, target, version) = (first.next().unwrap_or(""), first.next().unwrap_or(""), first.next().unwrap_or("HTTP/1.1"));
    let headers: Vec<&str> = lines.filter(|l| !l.is_empty()).collect();
    let Some(who) = caller(&headers) else {
        return answer(&mut c, "407 Proxy Authentication Required", "Proxy-Authenticate: Basic realm=\"tirith\"\r\n", "Tirith's proxy needs the credentials of HTTPS_PROXY.\n").await;
    };
    let connect = method.eq_ignore_ascii_case("CONNECT");
    let (hostport, path) = if connect {
        (target, String::new())
    } else {
        let Some(rest) = target.get(..7).filter(|s| s.eq_ignore_ascii_case("http://")).map(|_| &target[7..]) else {
            return answer(&mut c, "400 Bad Request", "", "Only CONNECT and http:// requests go through Tirith's proxy.\n").await;
        };
        let at = rest.find(['/', '?']).unwrap_or(rest.len());
        let path = &rest[at..];
        (&rest[..at], if path.starts_with('/') { path.to_string() } else { format!("/{path}") })
    };
    let Some((host, port)) = host_port(hostport, if connect { 443 } else { 80 }) else {
        return answer(&mut c, "400 Bad Request", "", "No destination host.\n").await;
    };
    let addrs: Vec<SocketAddr> = match blocked(&host) {
        Some(rule) => return refuse(&app, &mut c, &who, &host, port, &rule).await,
        None => match tokio::net::lookup_host((host.as_str(), port)).await {
            Ok(a) => a.collect(),
            Err(e) => return answer(&mut c, "502 Bad Gateway", "", &format!("cannot resolve {host}: {e}\n")).await,
        },
    };
    if let Some(rule) = addrs.iter().find_map(|a| blocked(&a.ip().to_string())) {
        return refuse(&app, &mut c, &who, &host, port, &rule).await;
    }
    record(&app, &who, &host, port, None);
    let mut up = match tokio::time::timeout(Duration::from_secs(15), TcpStream::connect(&addrs[..])).await {
        Ok(Ok(up)) => up,
        Ok(Err(e)) => return answer(&mut c, "502 Bad Gateway", "", &format!("cannot reach {host}:{port}: {e}\n")).await,
        Err(_) => return answer(&mut c, "504 Gateway Timeout", "", &format!("{host}:{port} did not answer\n")).await,
    };
    if connect {
        c.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").await.map_err(|e| e.to_string())?;
    } else {
        let mut out = format!("{method} {path} {version}\r\n");
        for h in &headers {
            let name = h.split(':').next().unwrap_or("").trim().to_ascii_lowercase();
            if !(name.starts_with("proxy-") || name == "connection" || name == "keep-alive") {
                out += &format!("{h}\r\n");
            }
        }
        out += "Connection: close\r\n\r\n";
        up.write_all(out.as_bytes()).await.map_err(|e| e.to_string())?;
    }
    up.write_all(&buf[end..]).await.map_err(|e| e.to_string())?;
    let _ = tokio::io::copy_bidirectional(&mut c, &mut up).await;
    Ok(())
}

async fn refuse(app: &Shared, c: &mut TcpStream, who: &str, host: &str, port: u16, rule: &str) -> Result<(), String> {
    record(app, who, host, port, Some(rule));
    let body = format!("Blocked by Tirith: {host} (rule {rule}). The owner blocked it; it is allowed again with: tirith net unblock {rule}\n");
    answer(c, "403 Forbidden", &format!("X-Tirith-Blocked: {rule}\r\n"), &body).await
}
