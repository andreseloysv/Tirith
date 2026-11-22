// A connector's sign-in (src/oauth/) end to end and offline: a fake server is the MCP server, its protected-resource and authorization-server metadata, its registration, authorization and token endpoints; a test daemon with its proxy; the browser a plain client following the redirect; the gateway's stdio loop in-process. No model, no sandbox, no network beyond 127.0.0.1.
use super::door::Door;
use super::fakes::stdio_server;
use super::tests::{call, hello, http, text};
use crate::app::{App, Shared};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::Json;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const LOCAL_SECRET: &str = "local-secret-value-2c81f0";
const MAIL: &str = "manager@bot-mail";
const OTHER: &str = "manager@bot-other";

#[derive(Default)]
struct Fake {
    port: u16,
    registered: Vec<String>,
    challenge: String,
    code: String,
    verified: bool,
    refreshes: u32,
    issued: Vec<String>,
    valid: String,
    refresh: String,
    /// Each MCP request's authorization header, "" for none.
    bearers: Vec<String>,
    resources: Vec<String>,
}

type Shared2 = Arc<Mutex<Fake>>;

fn decode(s: &str) -> String {
    let (mut out, b) = (Vec::new(), s.as_bytes());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                let hex = std::str::from_utf8(&b[i + 1..i + 3]).unwrap();
                out.push(u8::from_str_radix(hex, 16).unwrap());
                i += 2;
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8(out).unwrap()
}

fn form(body: &str) -> HashMap<String, String> {
    body.split('&')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (decode(k), decode(v)))
        .collect()
}

async fn resource(State(f): State<Shared2>) -> Json<Value> {
    let port = f.lock().unwrap().port;
    Json(json!({
        "resource": format!("http://127.0.0.1:{port}/v2/mcp"),
        "authorization_servers": [format!("http://127.0.0.1:{port}/as")],
        "scopes_supported": ["default"],
    }))
}

async fn metadata(State(f): State<Shared2>) -> Json<Value> {
    let base = format!("http://127.0.0.1:{}/as", f.lock().unwrap().port);
    Json(json!({
        "issuer": base,
        "authorization_endpoint": format!("{base}/authorize"),
        "token_endpoint": format!("{base}/token"),
        "registration_endpoint": format!("{base}/register"),
        "code_challenge_methods_supported": ["S256"],
    }))
}

async fn register(State(f): State<Shared2>, Json(b): Json<Value>) -> Json<Value> {
    let redirect = b["redirect_uris"][0].as_str().unwrap_or("").to_string();
    f.lock().unwrap().registered.push(redirect);
    Json(json!({"client_id": "fake-client-1"}))
}

async fn authorize(State(f): State<Shared2>, Query(q): Query<HashMap<String, String>>) -> Response {
    let mut f = f.lock().unwrap();
    let ok = q.get("response_type").is_some_and(|r| r == "code")
        && q.get("client_id").is_some_and(|c| c == "fake-client-1")
        && q.get("code_challenge_method").is_some_and(|m| m == "S256")
        && q.get("redirect_uri") == f.registered.first()
        && q.contains_key("state");
    if !ok {
        return (StatusCode::BAD_REQUEST, "bad authorize").into_response();
    }
    f.challenge = q["code_challenge"].clone();
    f.code = format!("code-{}", rand::random::<u32>());
    f.resources
        .push(q.get("resource").cloned().unwrap_or_default());
    let to = format!("{}?code={}&state={}", q["redirect_uri"], f.code, q["state"]);
    Redirect::to(&to).into_response()
}

fn issue(f: &mut Fake, lives: u64) -> Json<Value> {
    let n = f.issued.len() + 1;
    let access = format!("access-{n}-{:08x}", rand::random::<u32>());
    let refresh = format!("refresh-{n}-{:08x}", rand::random::<u32>());
    f.issued.push(access.clone());
    f.issued.push(refresh.clone());
    (f.valid, f.refresh) = (access.clone(), refresh.clone());
    Json(
        json!({"access_token": access, "refresh_token": refresh, "token_type": "Bearer", "expires_in": lives}),
    )
}

async fn token(State(f): State<Shared2>, body: String) -> Response {
    let q = form(&body);
    let mut f = f.lock().unwrap();
    f.resources
        .push(q.get("resource").cloned().unwrap_or_default());
    match q.get("grant_type").map(String::as_str) {
        Some("authorization_code") => {
            let verifier = q.get("code_verifier").cloned().unwrap_or_default();
            let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
            if q.get("code") != Some(&f.code) || challenge != f.challenge || f.code.is_empty() {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "invalid_grant"})),
                )
                    .into_response();
            }
            f.code.clear(); // one use
            f.verified = true;
            issue(&mut f, 1).into_response() // ends within the minute: the gateway renews it
        }
        Some("refresh_token") if q.get("refresh_token") == Some(&f.refresh) => {
            f.refreshes += 1;
            issue(&mut f, 3600).into_response()
        }
        _ => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid_grant"})),
        )
            .into_response(),
    }
}

async fn mcp(State(f): State<Shared2>, headers: HeaderMap, body: String) -> Response {
    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let valid = {
        let mut f = f.lock().unwrap();
        f.bearers.push(bearer.clone());
        !f.valid.is_empty() && bearer == format!("Bearer {}", f.valid)
    };
    if !valid {
        return (
            StatusCode::UNAUTHORIZED,
            [("www-authenticate", "Bearer")],
            "",
        )
            .into_response();
    }
    let msg: Value = serde_json::from_str(&body).unwrap_or_default();
    let reply = |result: Value| Json(json!({"jsonrpc": "2.0", "id": msg["id"], "result": result}));
    match msg["method"].as_str().unwrap_or("") {
        "initialize" => reply(json!({"protocolVersion": "2025-06-18", "capabilities": {"tools": {}}, "serverInfo": {"name": "fake-asana", "version": "1"}})).into_response(),
        "tools/list" => reply(json!({"tools": [{"name": "tasks", "description": "lists tasks", "inputSchema": {"type": "object"}}]})).into_response(),
        "tools/call" => reply(json!({"content": [{"type": "text", "text": "three tasks"}]})).into_response(),
        _ => StatusCode::ACCEPTED.into_response(),
    }
}

async fn fake() -> (u16, Shared2) {
    let f: Shared2 = Default::default();
    let router = axum::Router::new()
        .route(
            "/.well-known/oauth-protected-resource/v2/mcp",
            get(resource),
        )
        .route("/.well-known/oauth-authorization-server/as", get(metadata))
        .route("/as/register", post(register))
        .route("/as/authorize", get(authorize))
        .route("/as/token", post(token))
        .route("/v2/mcp", post(mcp))
        .with_state(f.clone());
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    f.lock().unwrap().port = port;
    tokio::spawn(async move { drop(axum::serve(listener, router).await) });
    (port, f)
}

fn free_port() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.local_addr().unwrap().port()
}

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("tirith-oauth-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn config(port: u16, proxy: u16, fake: u16, argv: &[String]) -> String {
    let argv = serde_json::to_string(argv).unwrap();
    format!(
        r#"[server]
port = {port}

[network]
sandbox = false
proxy_port = {proxy}

[[connectors]]
id = "asana"
url = "http://127.0.0.1:{fake}/v2/mcp"

[[connectors]]
id = "notes"
command = {argv}
secrets = ["MAIL_TOKEN"]

[roles.bot-mail]
extends = "manager"
scope = "own"
[roles.bot-mail.connectors.asana]
[roles.bot-mail.connectors.notes]

[roles.bot-other]
extends = "manager"
scope = "own"
[roles.bot-other.connectors.asana]

[limits]
batch_seconds = 3600
"#
    )
}

/// The gateway as `who`'s agent runs it with `--bot <bot>`: everything it wrote back.
async fn gateway(app: &Shared, port: u16, who: &str, bot: &str, msgs: &[Value]) -> String {
    let door = Door {
        port,
        token: vec![("X-Tirith-Token", crate::server::testing::token(app, who))],
    };
    let input: String = msgs.iter().map(|m| format!("{m}\n")).collect();
    let bot = bot.to_string();
    let run = move || {
        let mut out = Vec::new();
        super::stdio::serve(std::io::Cursor::new(input), &mut out, &door, &bot).unwrap();
        String::from_utf8(out).unwrap()
    };
    tokio::task::spawn_blocking(run).await.unwrap()
}

fn lines(out: &str) -> Vec<Value> {
    out.lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn connector_oauth_flow() {
    let (home, seen) = (dir("home"), dir("seen"));
    let (fake_port, f) = fake().await;
    let (port, proxy) = (free_port(), free_port());
    let argv = stdio_server(&home, &seen);
    std::fs::write(
        home.join("config.toml"),
        config(port, proxy, fake_port, &argv),
    )
    .unwrap();
    let app = App::open_at(home.clone());
    assert_eq!(app.cfg().port, port, "the fixture's port loads");
    tokio::spawn(crate::sandbox::serve_proxy(app.clone()));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .unwrap();
    crate::server::testing::serving(port, &app);
    let served = app.clone();
    tokio::spawn(
        async move { drop(axum::serve(listener, crate::server::router(served, port)).await) },
    );
    let owner = crate::server::testing::owner(port);
    let secrets = |bot: &'static str| {
        let (app, port) = (app.clone(), port);
        async move {
            let owner = crate::server::testing::owner(port);
            let (code, body) = http(
                port,
                &owner,
                "GET",
                &format!("/api/secrets/{bot}"),
                Value::Null,
            )
            .await;
            assert_eq!(code, 200, "{body}");
            drop(app);
            body
        }
    };

    // the owner's local secret; a reserved name is never the owner's to set
    let set = |name: &str, value: &str| json!({"name": name, "value": value});
    let (code, _) = http(
        port,
        &owner,
        "POST",
        "/api/secrets/mail",
        set("MAIL_TOKEN", LOCAL_SECRET),
    )
    .await;
    assert_eq!(code, 200);
    let (code, body) = http(
        port,
        &owner,
        "POST",
        "/api/secrets/mail",
        set("TIRITH_OAUTH_ASANA_ACCESS", "forged"),
    )
    .await;
    assert_eq!(code, 400, "a reserved name is refused: {body}");

    // only the owner begins a sign-in, and only of a remote connector
    let mail = crate::server::testing::token(&app, MAIL);
    let (code, _) = http(
        port,
        &mail,
        "POST",
        "/api/connectors/asana/login",
        json!({"bot": "mail"}),
    )
    .await;
    assert_eq!(code, 403, "a bot's run cannot sign in");
    let (code, body) = http(
        port,
        &owner,
        "POST",
        "/api/connectors/notes/login",
        json!({"bot": "mail"}),
    )
    .await;
    assert!(
        code == 400 && body.contains("needs no login"),
        "a local connector: {body}"
    );

    let (code, body) = http(
        port,
        &owner,
        "POST",
        "/api/connectors/asana/login",
        json!({"bot": "mail"}),
    )
    .await;
    assert_eq!(code, 200, "{body}");
    let begun: Value = serde_json::from_str(&body).unwrap();
    let (url, ticket) = (
        begun["url"].as_str().unwrap(),
        begun["ticket"].as_str().unwrap(),
    );
    let callback = format!("http://127.0.0.1:{port}/oauth/callback");
    assert_eq!(
        f.lock().unwrap().registered,
        [callback.as_str()],
        "registered for the 127.0.0.1 callback"
    );
    let parsed = reqwest::Url::parse(url).unwrap();
    let q: HashMap<String, String> = parsed.query_pairs().into_owned().collect();
    assert!(
        url.starts_with(&format!("http://127.0.0.1:{fake_port}/as/authorize?")),
        "{url}"
    );
    assert_eq!(q["code_challenge_method"], "S256");
    assert_eq!(q["redirect_uri"], callback);
    assert_eq!(
        q["resource"],
        format!("http://127.0.0.1:{fake_port}/v2/mcp")
    );
    assert_eq!(q["scope"], "default");
    assert!(q["state"].len() >= 32 && !q.contains_key("code_verifier"));
    let stands = |t: String| async move {
        let owner = crate::server::testing::owner(port);
        http(
            port,
            &owner,
            "GET",
            &format!("/api/connectors/logins/{t}"),
            Value::Null,
        )
        .await
        .1
    };
    assert!(stands(ticket.into()).await.contains("waiting"));

    // a forged state reaches nothing. Host is bare, as every test daemon's (the same-origin guard's port is the process's first)
    let browser = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let visit = |to: String| {
        let browser = browser.clone();
        async move {
            browser
                .get(to)
                .header("host", "127.0.0.1")
                .send()
                .await
                .unwrap()
        }
    };
    let forged = visit(format!("{callback}?code=x&state=forged")).await;
    assert_eq!(forged.status(), 400, "an unknown state is refused");

    // the owner's browser signs in: the server sends it back to the daemon's callback with the code
    let sent = browser.get(url).send().await.unwrap();
    assert_eq!(sent.status(), 303, "the server sends the browser back");
    let back = sent.headers()["location"].to_str().unwrap().to_string();
    assert!(back.starts_with(&format!("{callback}?code=")), "{back}");
    let signed = visit(back).await;
    assert_eq!(signed.status(), 200, "{}", signed.text().await.unwrap());
    assert!(stands(ticket.into()).await.contains("done"));
    assert!(
        f.lock().unwrap().verified,
        "the code came with the verifier of its challenge"
    );
    let again = visit(format!("{callback}?code=x&state={}", q["state"])).await;
    assert_eq!(again.status(), 400, "a state is used once");

    // kept under the reserved names in mail's store alone; no route answers a value
    let listed = secrets("mail").await;
    for name in [
        "TIRITH_OAUTH_ASANA_ACCESS",
        "TIRITH_OAUTH_ASANA_REFRESH",
        "TIRITH_OAUTH_ASANA_GRANT",
    ] {
        assert!(listed.contains(name), "{name} kept: {listed}");
    }
    assert_eq!(
        secrets("other").await,
        "[]",
        "another bot's store is untouched"
    );
    let first = f.lock().unwrap().issued.clone();
    assert!(first.iter().all(|t| !listed.contains(t.as_str())));

    // the gateway renews the access that ends within the minute, and sends it to asana alone
    let mut msgs = hello();
    msgs.push(call(3, "asana__tasks", json!({})));
    msgs.push(call(4, "notes__read_mail", json!({})));
    let out = gateway(&app, port, MAIL, "mail", &msgs).await;
    let replies = lines(&out);
    assert_eq!(text(&replies[2]), "three tasks", "{out}");
    assert_eq!(
        text(&replies[3]),
        "done",
        "the local connector runs on its own secret: {out}"
    );
    assert_eq!(
        std::fs::read_to_string(seen.join("env")).unwrap(),
        LOCAL_SECRET
    );
    let (refreshes, second) = {
        let f = f.lock().unwrap();
        (f.refreshes, f.valid.clone())
    };
    assert_eq!(refreshes, 1, "renewed before it ended");
    let stored = |name: &'static str| {
        let app = app.clone();
        async move {
            tokio::task::spawn_blocking(move || crate::secrets::value(&app, "mail", name).unwrap())
                .await
                .unwrap()
        }
    };
    assert_eq!(
        stored("TIRITH_OAUTH_ASANA_ACCESS").await.as_deref(),
        Some(second.as_str())
    );
    assert!(f
        .lock()
        .unwrap()
        .bearers
        .iter()
        .all(|b| b.is_empty() || *b == format!("Bearer {second}")));

    // the server drops the access: the gateway's 401 renews it once and goes on
    f.lock().unwrap().valid = "revoked".into();
    let out2 = gateway(&app, port, MAIL, "mail", &msgs).await;
    assert_eq!(text(&lines(&out2)[2]), "three tasks", "{out2}");
    let third = f.lock().unwrap().valid.clone();
    assert_eq!(f.lock().unwrap().refreshes, 2, "renewed on the 401");
    assert_eq!(
        stored("TIRITH_OAUTH_ASANA_ACCESS").await.as_deref(),
        Some(third.as_str())
    );

    // another bot granted the same connector holds no sign-in: it sends no token and gets no tools of it
    let before = f.lock().unwrap().bearers.len();
    let out3 = gateway(&app, port, OTHER, "other", &hello()).await;
    assert!(!out3.contains("asana__tasks"), "{out3}");
    let bearers = f.lock().unwrap().bearers[before..].to_vec();
    assert!(
        !bearers.is_empty() && bearers.iter().all(String::is_empty),
        "{bearers:?}"
    );
    assert_eq!(secrets("other").await, "[]", "its store is still empty");
    assert_eq!(
        f.lock().unwrap().refreshes,
        2,
        "mail's sign-in was not used for it"
    );

    // no token ever reached an agent: not the gateway's answers, nor any route's
    let issued = f.lock().unwrap().issued.clone();
    for t in &issued {
        for said in [&out, &out2, &out3, &body, &secrets("mail").await] {
            assert!(!said.contains(t.as_str()), "a token reached an agent: {t}");
        }
    }
    assert!(
        f.lock()
            .unwrap()
            .resources
            .iter()
            .all(|r| r.ends_with("/v2/mcp")),
        "each request named its resource"
    );
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&seen);
}
