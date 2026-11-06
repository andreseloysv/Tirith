//! Network sandbox. Every process Tirith starts (lane agents, the manager, CEO reviews, merge-queue checks) runs
//! inside a macOS `sandbox-exec` profile that denies outbound network except loopback and local sockets, with its
//! proxy variables pointing at the daemon's own HTTP/CONNECT proxy (`sandbox/proxy.rs`). The proxy authenticates
//! the caller (`who` + token), applies the block list, and records each new destination as an event.

mod proxy;

use crate::app::{now_iso, Shared};
use crate::client::{call, Flag};
use crate::config;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use base64::Engine;
use regex::Regex;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::IpAddr;
use std::sync::{LazyLock, OnceLock, RwLock};

/// A process to start: program, arguments and extra environment.
pub struct Launch {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

impl Launch {
    pub fn command(&self, app: &Shared) -> tokio::process::Command {
        let mut c = tokio::process::Command::new(&self.program);
        c.args(&self.args);
        if !self.env.iter().any(|(k, _)| k == "PATH") {
            c.env("PATH", app.child_path());
        }
        for (k, v) in &self.env {
            c.env(k, v);
        }
        c
    }
}

/// Outbound network only to this machine: the daemon's port, the proxy and local sockets.
const PROFILE: &str = "(version 1)\n(allow default)\n(deny network-outbound)\n\
(allow network-outbound (remote ip \"localhost:*\"))\n(allow network-outbound (remote unix-socket))\n";
const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";
const NO_PROXY: &str = "localhost,127.0.0.1,::1";
const DEFAULT_PORT: u16 = 47812;
const XCODE_FLAGS: &str = "-IDEPackageSupportDisableManifestSandbox=YES -IDEPackageSupportDisablePluginExecutionSandbox=YES";

/// The block list in memory (mirrors the `net_blocks` table) and the secret every proxy token derives from.
static BLOCKS: RwLock<Vec<String>> = RwLock::new(Vec::new());
static SECRET: OnceLock<String> = OnceLock::new();

pub fn proxy_port(app: &Shared) -> u16 {
    config::get(&app.cfg().raw, &["network", "proxy_port"]).unwrap_or(DEFAULT_PORT)
}

/// Ok when processes run sandboxed; why not otherwise.
fn enabled(app: &Shared) -> Result<(), String> {
    if !config::get::<bool>(&app.cfg().raw, &["network", "sandbox"]).unwrap_or(true) {
        Err("[network] sandbox = false in config.toml".into())
    } else if !cfg!(target_os = "macos") || !std::path::Path::new(SANDBOX_EXEC).exists() {
        Err("the sandbox needs macOS (sandbox-exec)".into())
    } else if std::env::var("TIRITH_SANDBOXED").as_deref() == Ok("1") {
        Err("the daemon itself runs in a sandbox and macOS cannot nest them (its own proxy settings stay)".into())
    } else {
        Ok(())
    }
}

/// The proxy password of `who`: derived from a random secret kept in the daemon's database, so processes started
/// before a daemon restart keep their access.
fn token(who: &str) -> String {
    let secret = SECRET.get().map(String::as_str).unwrap_or("");
    let digest = Sha256::digest(format!("{secret}\n{who}").as_bytes());
    digest.iter().take(16).map(|b| format!("{b:02x}")).collect()
}

/// `who` as a proxy user name: ':' is the user/password separator, so "merge:x" travels as "merge~x".
fn user(who: &str) -> String {
    who.replace(':', "~")
}

/// The lane and project an event of `who` belongs to ("merge:<lane>" is the lane's; "manager" stays itself).
fn owner(app: &Shared, who: &str) -> (String, String) {
    let id = who.strip_prefix("merge:").unwrap_or(who);
    (id.to_string(), app.lane(id).map(|l| l.project).unwrap_or_default())
}

/// Wraps `argv` for a process Tirith starts on behalf of `who` (a lane id, "manager", "ceo" or "merge:<lane>"):
/// `sandbox-exec -p <profile> argv…` with the proxy environment and the PATH with the tool wrappers first.
pub fn wrap(app: &Shared, who: &str, argv: Vec<String>) -> Launch {
    let mut it = argv.into_iter();
    let (program, args): (String, Vec<String>) = (it.next().unwrap_or_default(), it.collect());
    if let Err(why) = enabled(app) {
        let (lane, project) = owner(app, who);
        app.event(&lane, &project, "net-off", "sandbox", &format!("{who} runs without the network sandbox: {why}"), "");
        return Launch { program, args, env: vec![] };
    }
    let (port, user, token) = (proxy_port(app), user(who), token(who));
    let url = format!("http://{user}:{token}@127.0.0.1:{port}");
    let mut env: Vec<(String, String)> = ["HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY", "https_proxy", "http_proxy", "all_proxy"].iter().map(|k| (k.to_string(), url.clone())).collect();
    for (k, v) in [("NO_PROXY", NO_PROXY), ("no_proxy", NO_PROXY), ("NODE_USE_ENV_PROXY", "1"), ("TIRITH_SANDBOXED", "1")] {
        env.push((k.into(), v.into()));
    }
    // Gradle and the JVM ignore HTTPS_PROXY; the JDK refuses Basic proxy auth on tunnels unless re-enabled.
    let mut gradle = std::env::var("GRADLE_OPTS").unwrap_or_default();
    for scheme in ["http", "https"] {
        for (k, v) in [("proxyHost", "127.0.0.1"), ("proxyPort", &port.to_string()), ("proxyUser", &user), ("proxyPassword", &token), ("nonProxyHosts", "localhost|127.0.0.1")] {
            if !gradle.contains(&format!("-D{scheme}.{k}=")) {
                gradle += &format!(" -D{scheme}.{k}={v}");
            }
        }
    }
    gradle += " -Djdk.http.auth.tunneling.disabledSchemes= -Djdk.http.auth.proxying.disabledSchemes=";
    env.push(("GRADLE_OPTS".into(), gradle.trim().into()));
    // ssh ignores proxy variables: git over ssh goes through the proxy with `tirith net tunnel`.
    if std::env::var_os("GIT_SSH_COMMAND").is_none() {
        if let Ok(exe) = std::env::current_exe() {
            env.push(("GIT_SSH_COMMAND".into(), format!("ssh -o ProxyCommand='{} net tunnel %h %p'", exe.display())));
        }
    }
    env.push(("PATH".into(), format!("{}:{}", app.home.join("net/bin").display(), app.child_path())));
    env.push(("ZDOTDIR".into(), app.home.join("net/zdot").display().to_string()));
    let mut all = vec!["-p".to_string(), PROFILE.to_string(), program];
    all.extend(args);
    Launch { program: SANDBOX_EXEC.into(), args: all, env }
}

/// The wrappers in `<home>/net/bin`: tools that start a sandbox of their own (macOS refuses to nest them) get it
/// turned off, since the outer sandbox confines them already. Each wrapper takes its own folder off PATH.
fn write_wrappers(app: &Shared) {
    let dir = app.home.join("net/bin");
    let _ = std::fs::create_dir_all(&dir);
    let head = format!(
        "#!/bin/sh\n# Written by Tirith: inside its network sandbox, which macOS will not let a tool nest another in.\n\
         PATH=$(printf %s \"$PATH\" | tr : '\\n' | grep -vxF '{}' | paste -sd: -)\n",
        dir.display()
    );
    let swift = "case \"$1\" in build|test|run|package)\n  case \" $* \" in *\" --disable-sandbox \"*) exec swift \"$@\";; esac\n  \
                 sub=$1; shift; exec swift \"$sub\" --disable-sandbox \"$@\";;\nesac\nexec swift \"$@\"\n";
    let xcode = format!(
        "case \" $* \" in *\" -IDEPackageSupportDisableManifestSandbox\"*) exec xcodebuild \"$@\";; esac\n\
         case \" $* \" in *\" -scmProvider \"*) scm=;; *) scm='-scmProvider system';; esac\n\
         case \" $* \" in *\" OTHER_SWIFT_FLAGS=\"*) exec xcodebuild {XCODE_FLAGS} $scm \"$@\";; esac\n\
         exec xcodebuild {XCODE_FLAGS} $scm 'OTHER_SWIFT_FLAGS=$(inherited) -disable-sandbox' \"$@\"\n"
    );
    let mut files = vec![("swift".to_string(), swift.to_string()), ("xcodebuild".into(), xcode)];
    for tool in ["swift-build", "swift-test", "swift-run", "swift-package"] {
        files.push((tool.into(), format!("case \" $* \" in *\" --disable-sandbox \"*) exec {tool} \"$@\";; esac\nexec {tool} --disable-sandbox \"$@\"\n")));
    }
    for (name, body) in files {
        let path = dir.join(name);
        let text = format!("{head}{body}");
        if std::fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
            let _ = std::fs::write(&path, text);
        }
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
    }
    // Agents run commands in zsh login shells, where macOS's path_helper puts /usr/bin before the wrappers again:
    // the ZDOTDIR given to sandboxed processes loads the owner's own startup files, then puts the wrappers first.
    let zdot = app.home.join("net/zdot");
    let _ = std::fs::create_dir_all(&zdot);
    for file in [".zshenv", ".zprofile", ".zshrc", ".zlogin"] {
        let text = format!("[ -f \"$HOME/{file}\" ] && . \"$HOME/{file}\"\nexport PATH=\"{}:$PATH\"\n", dir.display());
        let _ = std::fs::write(zdot.join(file), text);
    }
}

/// `https://api.example.com:443/x` -> `api.example.com`; `*.x.com` -> `x.com`; addresses and ranges stay.
fn clean(rule: &str) -> String {
    let mut t = rule.trim().to_lowercase();
    if let Some((_, rest)) = t.split_once("://") {
        t = rest.to_string();
    }
    if cidr(&t).is_none() {
        t = t.split(['/', '?', '#']).next().unwrap_or("").to_string();
    }
    if t.matches(':').count() == 1 {
        t = t.split(':').next().unwrap_or("").to_string();
    }
    t.trim_start_matches("*.").trim_matches(['.', '[', ']']).to_string()
}

/// An address or a CIDR range: the network address and its prefix length.
fn cidr(text: &str) -> Option<(IpAddr, u32)> {
    let (a, b) = text.split_once('/').unwrap_or((text, ""));
    let ip: IpAddr = a.trim_matches(['[', ']']).parse().ok()?;
    let max = if ip.is_ipv4() { 32 } else { 128 };
    let bits = if b.is_empty() { max } else { b.parse().ok().filter(|n| *n <= max)? };
    Some((ip, bits))
}

fn in_range(ip: IpAddr, (net, bits): (IpAddr, u32)) -> bool {
    let mask = |w: u32| if bits == 0 { 0 } else { u128::MAX << (w - bits) };
    match (ip, net) {
        (IpAddr::V4(a), IpAddr::V4(n)) => (u32::from(a) as u128 ^ u32::from(n) as u128) & mask(32) & 0xffff_ffff == 0,
        (IpAddr::V6(a), IpAddr::V6(n)) => (u128::from(a) ^ u128::from(n)) & mask(128) == 0,
        (IpAddr::V6(a), IpAddr::V4(_)) => a.to_ipv4_mapped().is_some_and(|v4| in_range(IpAddr::V4(v4), (net, bits))),
        _ => false,
    }
}

/// The rule that blocks `host` (a domain and its subdomains, or an address inside a rule's range), else `None`.
pub fn blocked(host: &str) -> Option<String> {
    let host = host.trim().trim_matches(['.', '[', ']']).to_lowercase();
    let ip: Option<IpAddr> = host.parse().ok();
    BLOCKS
        .read()
        .unwrap()
        .iter()
        .find(|rule| match (ip, cidr(rule)) {
            (Some(ip), Some(range)) => in_range(ip, range),
            (None, None) => host == **rule || host.ends_with(&format!(".{rule}")),
            _ => false,
        })
        .cloned()
}

static HOSTS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)(?:^|[^a-z0-9_.-])((?:[a-z0-9-]+\.)+[a-z0-9-]+|\[[0-9a-f:.]+\])").unwrap());

/// For agent sessions NOT started by Tirith (the owner's own sessions): the refusal when a tool call names a
/// blocked destination (a URL or host in a Bash command, a WebFetch url), else `None`.
pub fn check_command(tool: &str, input: &Value) -> Option<String> {
    if BLOCKS.read().unwrap().is_empty() {
        return None;
    }
    let text = match tool {
        "Bash" => input["command"].as_str().filter(|c| !c.contains("tirith net "))?,
        "WebFetch" => input["url"].as_str()?,
        _ => return None,
    };
    HOSTS.captures_iter(text).find_map(|c| {
        blocked(&c[1]).map(|rule| {
            format!(
                "{} is blocked by the owner (rule {rule}): Tirith refuses calls to it. Use another source, or ask the \
         owner to allow it again (tirith net unblock {rule}, or Desbloquear in the Red section of the page).",
                &c[1]
            )
        })
    })
}

static CRED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"[^\s'";|&<>]*/\.(?:ssh|aws|gnupg|kube|docker)(?:/[^\s'";|&<>]*)?|[^\s'";|&<>]*\.config/gh(?:/[^\s'";|&<>]*)?|(?:^|[\s/'"=])\.env(?:\.[\w-]+)*\b|[^\s'";|&<>]*\.(?:netrc|git-credentials|npmrc|pypirc)\b|\bsecurity\s+(?:find-[a-z-]+|dump-keychain)\b[^;&|\n]*"#).unwrap()
});
static GIT_PUSH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bgit\b[^;&|\n]*?\b(push\b[^;&|\n]*|remote\s+(?:add|set-url)\b[^;&|\n]*)").unwrap());
static GH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:^|[;&|(\n]\s*|\bxargs\s+)gh\s+([a-z-]+(?:\s+[a-z-]+)?)").unwrap());
static ENV: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$\{?([A-Z][A-Z0-9_]{2,})|\bprintenv\s+([A-Z][A-Z0-9_]{2,})").unwrap());
const ENV_SKIP: [&str; 8] = ["PATH", "HOME", "USER", "SHELL", "PWD", "TMPDIR", "LANG", "TERM"];

/// Access-audit tags of one tool call: `(category, target)` with category one of "credential", "git_push",
/// "gh", "env" (target = variable names, never values). Recorded by the hook handler as `access` events.
pub fn tags(tool: &str, input: &Value) -> Vec<(&'static str, String)> {
    let mut out = vec![];
    if tool != "Bash" {
        let file = ["file_path", "path", "notebook_path"].iter().find_map(|k| input[*k].as_str()).unwrap_or("");
        if !file.is_empty() && CRED.is_match(file) && !file.ends_with(".example") {
            out.push(("credential", file.to_string()));
        }
        return out;
    }
    let cmd = input["command"].as_str().unwrap_or("");
    for m in CRED.find_iter(cmd) {
        let t = m.as_str().trim_start_matches([' ', '\'', '"', '=']).trim().to_string();
        if !t.ends_with(".example") && !out.iter().any(|(_, x)| *x == t) {
            out.push(("credential", t));
        }
    }
    out.extend(GIT_PUSH.captures_iter(cmd).map(|c| ("git_push", format!("git {}", c[1].trim()))));
    out.extend(GH.captures_iter(cmd).map(|c| ("gh", format!("gh {}", c[1].trim()))));
    let mut names: Vec<&str> = vec![];
    for c in ENV.captures_iter(cmd) {
        let n = c.get(1).or(c.get(2)).map(|m| m.as_str()).unwrap_or("");
        if !ENV_SKIP.contains(&n) && !n.starts_with("TIRITH") && !names.contains(&n) {
            names.push(n);
        }
    }
    if !names.is_empty() {
        out.push(("env", names.join(", ")));
    }
    out
}

/// Starts the proxy inside the daemon, with the block list, the token secret and the tool wrappers ready.
pub fn start(app: Shared) {
    {
        let db = app.db.lock().unwrap();
        let _ = db.execute("CREATE TABLE IF NOT EXISTS net_blocks (rule TEXT PRIMARY KEY, at TEXT)", []);
        if let Ok(mut q) = db.prepare("SELECT rule FROM net_blocks ORDER BY at") {
            let rules: Vec<String> = q.query_map([], |r| r.get(0)).map(|r| r.flatten().collect()).unwrap_or_default();
            *BLOCKS.write().unwrap() = rules;
        };
    }
    let secret = app.kv("net_secret").unwrap_or_else(|| {
        let s: String = rand::random::<[u8; 32]>().iter().map(|b| format!("{b:02x}")).collect();
        app.set_kv("net_secret", Some(&s));
        s
    });
    let _ = SECRET.set(secret);
    write_wrappers(&app);
    proxy::seed(&app);
    tokio::spawn(proxy::serve(app));
}

fn block(app: &Shared, rule: &str, on: bool) -> Result<String, String> {
    let rule = clean(rule);
    let domain = rule.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    if rule.is_empty() || !(domain || cidr(&rule).is_some()) {
        return Err(format!("not a domain, address or range: {rule:?}"));
    }
    let db = app.db.lock().unwrap();
    let mut list = BLOCKS.write().unwrap();
    list.retain(|r| *r != rule);
    if on {
        let _ = db.execute("INSERT OR REPLACE INTO net_blocks (rule, at) VALUES (?1, ?2)", [&rule, &now_iso()]);
        list.push(rule.clone());
    } else {
        let _ = db.execute("DELETE FROM net_blocks WHERE rule = ?1", [&rule]);
    }
    drop((db, list));
    app.emit("net", json!({"blocks": *BLOCKS.read().unwrap()}));
    Ok(rule)
}

pub fn routes() -> Router<Shared> {
    type R = Result<Json<Value>, (StatusCode, String)>;
    async fn change(app: Shared, b: Value, on: bool) -> R {
        let rule = block(&app, b["rule"].as_str().unwrap_or(""), on).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
        Ok(Json(json!({"rule": rule, "blocks": *BLOCKS.read().unwrap()})))
    }
    Router::new()
        .route(
            "/api/net/status",
            get(|State(app): State<Shared>| async move {
                Json(json!({"sandbox": enabled(&app).is_ok(), "why": enabled(&app).err(), "proxy_port": proxy_port(&app),
                "proxy": proxy::state(), "blocks": *BLOCKS.read().unwrap()}))
            }),
        )
        .route(
            "/api/net/blocks",
            get(|| async { Json(json!(*BLOCKS.read().unwrap())) })
                .post(|State(app): State<Shared>, Json(b): Json<Value>| change(app, b, true))
                .delete(|State(app): State<Shared>, Json(b): Json<Value>| change(app, b, false)),
        )
        .route("/api/net/destinations", get(|| async { Json(proxy::destinations()) }))
}

/// `tirith net …` commands; `None` when `pos` is not one of this module's commands.
pub fn cli(pos: &[&str], _flag: Flag) -> Option<Result<(), String>> {
    let parse = |r: Result<String, String>| r.and_then(|b| serde_json::from_str::<Value>(&b).map_err(|e| e.to_string()));
    Some(match pos {
        ["net", "status"] => parse(call("GET", "/api/net/status", None)).map(|s| {
            let on = if s["sandbox"] == true { "on".to_string() } else { format!("off ({})", s["why"].as_str().unwrap_or("")) };
            println!("sandbox: {on}\nproxy: 127.0.0.1:{} {}", s["proxy_port"], s["proxy"].as_str().unwrap_or(""));
        }),
        ["net", "list"] => parse(call("GET", "/api/net/destinations", None)).and_then(|d| {
            for x in d.as_array().into_iter().flatten().take(40) {
                let lanes: Vec<&str> = x["lanes"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
                let rule = x["rule"].as_str().map(|r| format!("  BLOCKED by {r}")).unwrap_or_default();
                let n = format!("{}{}", x["count"], if x["refused"] == 0 { String::new() } else { format!("+{} refused", x["refused"]) });
                println!("{:<40} {n:>12}  {}  {}{rule}", x["host"].as_str().unwrap_or(""), x["last"].as_str().unwrap_or(""), lanes.join(","));
            }
            let blocks = parse(call("GET", "/api/net/blocks", None))?;
            println!("blocked: {}", blocks.as_array().map(|b| b.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" ")).unwrap_or_default());
            Ok(())
        }),
        ["net", "block", rule] => parse(call("POST", "/api/net/blocks", Some(json!({"rule": rule})))).map(|v| println!("blocked {}", v["rule"].as_str().unwrap_or(""))),
        ["net", "unblock", rule] => parse(call("DELETE", "/api/net/blocks", Some(json!({"rule": rule})))).map(|v| println!("unblocked {}", v["rule"].as_str().unwrap_or(""))),
        ["net", "tunnel", host, port] => tunnel(host, port),
        ["net", ..] => Err("tirith net status|list|block <domain|ip|cidr>|unblock <rule>|tunnel <host> <port>".into()),
        _ => return None,
    })
}

/// `tirith net tunnel <host> <port>`: stdin/stdout through the proxy (ssh's ProxyCommand inside the sandbox).
fn tunnel(host: &str, port: &str) -> Result<(), String> {
    let url = std::env::var("HTTPS_PROXY").map_err(|_| "no HTTPS_PROXY: the tunnel works inside Tirith's sandbox")?;
    let rest = url.strip_prefix("http://").ok_or("HTTPS_PROXY is not an http:// proxy")?.trim_end_matches('/');
    let (cred, addr) = rest.rsplit_once('@').ok_or("HTTPS_PROXY has no credentials")?;
    let mut s = std::net::TcpStream::connect(addr).map_err(|e| format!("proxy {addr}: {e}"))?;
    let auth = base64::engine::general_purpose::STANDARD.encode(cred);
    write!(s, "CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\nProxy-Authorization: Basic {auth}\r\n\r\n").map_err(|e| e.to_string())?;
    let (mut head, mut b) = (Vec::new(), [0u8; 1]);
    while !head.ends_with(b"\r\n\r\n") && head.len() < 16384 && s.read(&mut b).map_err(|e| e.to_string())? == 1 {
        head.push(b[0]);
    }
    let head = String::from_utf8_lossy(&head);
    if !head.starts_with("HTTP/1.1 200") && !head.starts_with("HTTP/1.0 200") {
        return Err(format!("proxy refused {host}:{port}: {}", head.lines().next().unwrap_or("no answer")));
    }
    let mut up = s.try_clone().map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let _ = std::io::copy(&mut std::io::stdin(), &mut up);
        let _ = up.shutdown(std::net::Shutdown::Write);
    });
    let (mut buf, mut out) = ([0u8; 16384], std::io::stdout());
    loop {
        match s.read(&mut buf) {
            Ok(0) | Err(_) => return Ok(()),
            Ok(n) => out.write_all(&buf[..n]).and_then(|_| out.flush()).map_err(|e| e.to_string())?,
        }
    }
}
