//! Reaching the destination: refused if blocked by name or by any of its addresses.
use super::{
    answer::{answer, refuse},
    record::record,
};
use crate::{app::Shared, sandbox::blocks::blocked};
use std::{net::SocketAddr, time::Duration};
use tokio::net::TcpStream;

/// The connection to `host:port`; `None` once the caller got the refusal or the error.
pub async fn upstream(
    app: &Shared,
    c: &mut TcpStream,
    who: &str,
    host: &str,
    port: u16,
) -> Result<Option<TcpStream>, String> {
    let closed =
        crate::lab::closed(who).then(|| "replay (a replay reads nothing outside)".to_string());
    let addrs: Vec<SocketAddr> = match closed.or_else(|| blocked(host)) {
        Some(rule) => return refuse(app, c, who, host, port, &rule).await.map(|_| None),
        None => match tokio::net::lookup_host((host, port)).await {
            Ok(a) => a.collect(),
            Err(e) => {
                let body = format!("cannot resolve {host}: {e}\n");
                return answer(c, "502 Bad Gateway", "", &body).await.map(|_| None);
            }
        },
    };
    if let Some(rule) = addrs.iter().find_map(|a| blocked(&a.ip().to_string())) {
        return refuse(app, c, who, host, port, &rule).await.map(|_| None);
    }
    record(app, who, host, port, None);
    connect(c, &addrs, host, port).await
}

/// Connects within 15 s, else tells the caller why not.
async fn connect(
    c: &mut TcpStream,
    addrs: &[SocketAddr],
    host: &str,
    port: u16,
) -> Result<Option<TcpStream>, String> {
    let (status, body) =
        match tokio::time::timeout(Duration::from_secs(15), TcpStream::connect(addrs)).await {
            Ok(Ok(up)) => return Ok(Some(up)),
            Ok(Err(e)) => (
                "502 Bad Gateway",
                format!("cannot reach {host}:{port}: {e}\n"),
            ),
            Err(_) => (
                "504 Gateway Timeout",
                format!("{host}:{port} did not answer\n"),
            ),
        };
    answer(c, status, "", &body).await.map(|_| None)
}
