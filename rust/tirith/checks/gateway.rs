// The connector gateway (src/gateway/): a test daemon with its proxy, a fake stdio and a fake HTTP MCP server, the gateway's stdio loop driven in-process, no model and no sandbox.
use super::door::Door;
use super::fakes::{http_server, stdio_server, Hits};
use crate::app::{App, Shared};
use serde_json::{json, Value};
use std::path::PathBuf;

pub(super) const MAIL_SECRET: &str = "mail-secret-value-5d1e9a";
const WEB_SECRET: &str = "web-secret-value-77b0c2";
const ORG_SECRET: &str = "org-secret-value-3fa418";
const SECRETS: [&str; 3] = [MAIL_SECRET, WEB_SECRET, ORG_SECRET];
pub(super) const BOT: &str = "manager@bot-mail";

pub(super) struct World {
    pub(super) app: Shared,
    pub(super) port: u16,
    home: PathBuf,
    seen: PathBuf,
    hits: Hits,
    web: u16,
    proxy: u16,
}

impl Drop for World {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.home);
        let _ = std::fs::remove_dir_all(&self.seen);
    }
}

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("tirith-gw-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A grant of mail to bot-mail: `extra` narrows it more.
fn config(argv: &[String], web: u16, proxy: u16, extra: &str) -> String {
    let argv = serde_json::to_string(argv).unwrap();
    format!(
        r#"[network]
sandbox = false
proxy_port = {proxy}

[[connectors]]
id = "mail"
command = {argv}
deny = ["delete_*"]
approval = ["send_*"]
secrets = ["MAIL_TOKEN"]

[[connectors]]
id = "web"
url = "http://127.0.0.1:{web}/mcp"
secrets = ["WEB_KEY", "WEB_ORG"]

[[connectors]]
id = "vault"
command = {argv}
blocked = true

[roles.bot-mail]
extends = "manager"
scope = "own"
[roles.bot-mail.connectors.mail]
tools = ["read_*", "send_*", "delete_*"]
{extra}
[roles.bot-mail.connectors.web]
[roles.bot-mail.connectors.vault]

[roles.bot-other]
extends = "manager"
scope = "own"

[limits]
batch_seconds = 3600
"#
    )
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// The daemon of a fresh home, its proxy listening, bot-mail's secrets set by the owner.
pub(super) async fn world(name: &str) -> World {
    let (home, seen) = (dir(name), dir(&format!("{name}-seen")));
    let (web, hits) = http_server().await;
    let argv = stdio_server(&home, &seen);
    let proxy = free_port();
    std::fs::write(home.join("config.toml"), config(&argv, web, proxy, "")).unwrap();
    let app = App::open_at(home.clone());
    assert_eq!(
        app.cfg().factory.granted("bot-mail").len(),
        3,
        "the fixture's grants load"
    );
    tokio::spawn(crate::sandbox::serve_proxy(app.clone()));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    crate::server::testing::serving(port, &app);
    let served = app.clone();
    tokio::spawn(
        async move { drop(axum::serve(listener, crate::server::router(served, port)).await) },
    );
    let owner = crate::server::testing::owner(port);
    for (name, value) in [
        ("MAIL_TOKEN", MAIL_SECRET),
        ("WEB_KEY", WEB_SECRET),
        ("WEB_ORG", ORG_SECRET),
    ] {
        let (code, _) = http(
            port,
            &owner,
            "POST",
            "/api/secrets/mail",
            json!({"name": name, "value": value}),
        )
        .await;
        assert_eq!(code, 200, "the owner sets {name}");
    }
    World {
        app,
        port,
        home,
        seen,
        hits,
        web,
        proxy,
    }
}

/// One request to the test daemon with `token`: its status and its body.
pub(super) async fn http(
    port: u16,
    token: &str,
    method: &str,
    path: &str,
    body: Value,
) -> (u16, String) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let mut req = client
        .request(method.parse().unwrap(), url)
        .header("host", "127.0.0.1");
    req = req.header("x-tirith-token", token);
    if !body.is_null() {
        req = req.json(&body);
    }
    let resp = req.send().await.unwrap();
    (resp.status().as_u16(), resp.text().await.unwrap())
}

impl World {
    pub(super) fn token(&self, who: &str) -> String {
        crate::server::testing::token(&self.app, who)
    }

    /// The gateway as the agent of `who` runs it with `--bot <bot>`, fed `msgs`: what it wrote back, or why it refused.
    pub(super) async fn gateway(
        &self,
        who: &str,
        bot: &str,
        msgs: &[Value],
    ) -> Result<Vec<Value>, String> {
        let door = Door {
            port: self.port,
            token: vec![("X-Tirith-Token", self.token(who))],
        };
        let input: String = msgs.iter().map(|m| format!("{m}\n")).collect();
        let bot = bot.to_string();
        let run = move || {
            let mut out = Vec::new();
            super::stdio::serve(std::io::Cursor::new(input), &mut out, &door, &bot)?;
            Ok::<String, String>(String::from_utf8(out).unwrap())
        };
        let out = tokio::task::spawn_blocking(run).await.unwrap()?;
        Ok(out
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect())
    }

    /// What the stdio server got as tools calls.
    pub(super) fn calls(&self) -> String {
        std::fs::read_to_string(self.seen.join("calls")).unwrap_or_default()
    }

    /// The recorded attempts, oldest first.
    pub(super) fn rows(&self) -> Vec<Value> {
        let cols = [
            "at",
            "bot",
            "conv",
            "turn",
            "connector",
            "tool",
            "args_sha",
            "result_sha",
            "verdict",
        ];
        let sql = format!(
            "SELECT {} FROM connector_calls ORDER BY id",
            cols.join(", ")
        );
        self.app.objects(&sql, [], &cols)
    }
}

fn request(id: u64, method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

pub(super) fn call(id: u64, name: &str, args: Value) -> Value {
    request(id, "tools/call", json!({"name": name, "arguments": args}))
}

/// The opening every agent sends.
pub(super) fn hello() -> Vec<Value> {
    vec![
        request(
            1,
            "initialize",
            json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "agent"}}),
        ),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        request(2, "tools/list", json!({})),
    ]
}

pub(super) fn text(reply: &Value) -> String {
    reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

fn names(reply: &Value) -> Vec<String> {
    let tools = reply["result"]["tools"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut names: Vec<String> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    names
}

#[tokio::test(flavor = "multi_thread")]
async fn mcp_gateway_scopes_tools() {
    let w = world("scopes").await;
    let other = w
        .gateway("manager@bot-other", "mail", &hello())
        .await
        .unwrap_err();
    assert!(
        other.contains("not 'mail'"),
        "another bot's run is refused: {other}"
    );
    let lane = w.gateway("some-lane", "mail", &hello()).await.unwrap_err();
    assert!(
        lane.contains("bot run's alone"),
        "a lane is refused: {lane}"
    );
    let manager = w
        .gateway("manager@manager-general", "mail", &hello())
        .await
        .unwrap_err();
    assert!(
        manager.contains("bot run's alone"),
        "a manager is refused: {manager}"
    );
    let owner = crate::server::testing::owner(w.port);
    let (code, _) = http(
        w.port,
        &owner,
        "POST",
        "/api/mcp/open",
        json!({"bot": "mail"}),
    )
    .await;
    assert_eq!(code, 403, "the owner opens no gateway");
    assert!(
        w.rows().is_empty() && w.calls().is_empty(),
        "a refused opening reaches nothing"
    );

    let mut msgs = hello();
    msgs.push(call(3, "mail__read_mail", json!({"q": "invoices"})));
    msgs.push(call(4, "web__fetch", json!({"page": 1})));
    let out = w.gateway(BOT, "mail", &msgs).await.unwrap();
    assert_eq!(
        out.len(),
        4,
        "every request answered, the notification not: {out:?}"
    );
    assert_eq!(out[0]["result"]["serverInfo"]["name"], "tirith-connectors");
    let listed = names(&out[1]);
    assert_eq!(
        listed,
        [
            "mail__read_mail",
            "mail__send_mail",
            "web__fetch",
            "web__post"
        ],
        "deny, scope and block hide the rest"
    );
    let tools = out[1]["result"]["tools"].as_array().unwrap();
    let send = tools
        .iter()
        .find(|t| t["name"] == "mail__send_mail")
        .unwrap();
    assert!(send["description"]
        .as_str()
        .unwrap()
        .starts_with("(asks the owner first)"));
    assert_eq!(text(&out[2]), "done", "a local tool ran: {:?}", out[2]);
    assert_eq!(text(&out[3]), "fetched", "a remote tool ran: {:?}", out[3]);
    assert!(w.calls().contains("read_mail") && w.calls().contains("invoices"));

    // The remote went out through the proxy as the bot: the proxy counted it under the bot, never the manager role it runs on.
    let net: Vec<String> = w.app.query(
        "SELECT lane FROM events WHERE kind = 'net' AND summary = ?1",
        [format!("127.0.0.1:{}", w.web)],
        |r| r.get(0),
    );
    assert_eq!(
        net,
        ["bot-mail"],
        "the proxy saw the bot's run reach the server"
    );
    let url = crate::sandbox::proxy_url(&w.app, BOT);
    let password = url.split(':').nth(2).unwrap().split('@').next().unwrap();
    assert_eq!(
        crate::sandbox::who_of(&w.app, password).as_deref(),
        Some(BOT),
        "its credentials are the bot's"
    );

    // The list is not trusted: a grant narrowed after it refuses the next call of a listed tool.
    let bot = w.token(BOT);
    let (code, opened) = http(
        w.port,
        &bot,
        "POST",
        "/api/mcp/open",
        json!({"bot": "mail"}),
    )
    .await;
    assert_eq!(code, 200, "{opened}");
    let session = serde_json::from_str::<Value>(&opened).unwrap()["session"]
        .as_str()
        .unwrap()
        .to_string();
    let argv = stdio_server(&w.home, &w.seen);
    std::fs::write(
        w.home.join("config.toml"),
        config(&argv, w.web, w.proxy, "deny = [\"read_*\"]"),
    )
    .unwrap();
    assert!(w.app.reload_config(true));
    let path = format!("/api/mcp/{session}/call");
    let (_, said) = http(
        w.port,
        &bot,
        "POST",
        &path,
        json!({"name": "mail__read_mail", "arguments": {"q": "again"}}),
    )
    .await;
    assert!(
        said.contains("refused by deny") && !w.calls().contains("again"),
        "{said}"
    );
    let other = w.token("manager@bot-other");
    let (code, _) = http(w.port, &other, "POST", &path, json!({"name": "web__fetch"})).await;
    assert_eq!(code, 404, "another run never finds this gateway");
    let (code, _) = http(
        w.port,
        &bot,
        "DELETE",
        &format!("/api/mcp/{session}"),
        Value::Null,
    )
    .await;
    assert_eq!(code, 200);
}

#[tokio::test(flavor = "multi_thread")]
async fn gateway_hides_secrets() {
    let w = world("secrets").await;
    let mut msgs = hello();
    msgs.push(call(3, "mail__read_mail", json!({"q": "x"})));
    msgs.push(call(4, "web__fetch", json!({})));
    msgs.push(call(5, "mail__delete_mail", json!({})));
    let out = w.gateway(BOT, "mail", &msgs).await.unwrap();
    assert_eq!(
        (text(&out[2]).as_str(), text(&out[3]).as_str()),
        ("done", "fetched")
    );

    // They reach the connectors, and only there.
    assert_eq!(
        std::fs::read_to_string(w.seen.join("env")).unwrap(),
        MAIL_SECRET,
        "in the local server's environment"
    );
    let hits = w.hits.lock().unwrap().clone();
    assert!(!hits.is_empty());
    for (headers, _) in &hits {
        let header = |k: &str| headers.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
        assert_eq!(
            header("authorization"),
            Some(format!("Bearer {WEB_SECRET}")),
            "in the remote's auth header"
        );
        assert_eq!(header("web-org").as_deref(), Some(ORG_SECRET));
    }
    assert!(hits.iter().skip(1).all(|(h, _)| h
        .iter()
        .any(|(k, v)| k == "mcp-session-id" && v == "web-session-1")));

    // Nowhere the agent reads or Tirith keeps.
    let bot = w.token(BOT);
    let (_, opened) = http(
        w.port,
        &bot,
        "POST",
        "/api/mcp/open",
        json!({"bot": "mail"}),
    )
    .await;
    let mut seen = vec![serde_json::to_string(&out).unwrap(), opened, w.calls()];
    seen.push(serde_json::to_string(&w.rows()).unwrap());
    seen.push(serde_json::to_string(&hits.iter().map(|(_, b)| b).collect::<Vec<_>>()).unwrap());
    for alias in ["anthropic", "codex", "muse"] {
        let args = crate::models::gateway_args_for(&w.app, alias, "bot-mail");
        let line = args.join(" ");
        assert!(
            line.contains("tirith_connectors") && line.contains("--bot") && line.contains("mail"),
            "{alias}: {args:?}"
        );
        seen.push(args.join(" "));
        for role in ["manager", "bot-other", "ceo"] {
            assert!(
                crate::models::gateway_args_for(&w.app, alias, role).is_empty(),
                "{alias} {role} gets no gateway"
            );
        }
    }
    let anthropic = crate::models::gateway_args_for(&w.app, "anthropic", "bot-mail");
    let config: Value =
        serde_json::from_str(anthropic[0].strip_prefix("--mcp-config=").unwrap()).unwrap();
    let env = config["mcpServers"]["tirith_connectors"]["env"]
        .as_object()
        .unwrap();
    assert_eq!(
        env.keys().collect::<Vec<_>>(),
        ["TIRITH_HOME", "TIRITH_TOKEN"],
        "the gateway's env names no secret"
    );
    assert_eq!(env["TIRITH_TOKEN"], "${TIRITH_TOKEN}");
    seen.extend(std::env::vars().map(|(k, v)| format!("{k}={v}")));
    seen.extend(files(&w.home));
    for said in &seen {
        for secret in SECRETS {
            assert!(!said.contains(secret), "a secret leaked: {said}");
        }
    }
}

/// Every file of `dir`, read as text.
fn files(dir: &std::path::Path) -> Vec<String> {
    let entries = std::fs::read_dir(dir).into_iter().flatten().flatten();
    entries
        .flat_map(|e| match e.path().is_dir() {
            true => files(&e.path()),
            false => vec![
                String::from_utf8_lossy(&std::fs::read(e.path()).unwrap_or_default()).to_string(),
            ],
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn refused_calls_are_recorded() {
    let w = world("records").await;
    let mut msgs = hello();
    msgs.push(call(3, "mail__delete_mail", json!({"id": 7})));
    msgs.push(call(4, "mail__delete_mail", json!({"id": 7})));
    msgs.push(call(5, "mail__label", json!({})));
    msgs.push(call(6, "vault__open", json!({})));
    msgs.push(call(7, "mail__send_mail", json!({"to": "a@b.c"})));
    msgs.push(call(
        8,
        "mail__read_mail",
        json!({"b": 2, "a": [1, {"z": 0, "y": 1}]}),
    ));
    msgs.push(call(9, "nowhere__x", json!({})));
    let out = w.gateway(BOT, "mail", &msgs).await.unwrap();
    let said: Vec<String> = out[2..].iter().map(text).collect();
    let words = [
        "refused by deny",
        "refused by deny",
        "refused by scope",
        "refused by block",
        "approval required",
        "done",
        "refused by scope",
    ];
    for (said, word) in said.iter().zip(words) {
        assert!(said.contains(word), "{said} says {word}");
    }
    assert!(out[2..]
        .iter()
        .enumerate()
        .all(|(i, r)| (r["result"]["isError"] == true) == (i != 5)));
    assert!(
        !w.calls().contains("delete_mail") && !w.calls().contains("send_mail"),
        "a refusal or an approval match never runs"
    );
    assert!(w.calls().contains("read_mail"));

    let rows = w.rows();
    assert_eq!(
        rows.len(),
        7,
        "every attempt, the repeated one twice: {rows:?}"
    );
    let verdicts: Vec<&str> = rows
        .iter()
        .map(|r| r["verdict"].as_str().unwrap())
        .collect();
    let expected = [
        "refused by deny",
        "refused by deny",
        "refused by scope",
        "refused by block",
        "waiting for approval",
        "ran",
        "refused by scope",
    ];
    assert_eq!(verdicts, expected);
    let tools: Vec<(&str, &str)> = rows
        .iter()
        .map(|r| {
            (
                r["connector"].as_str().unwrap(),
                r["tool"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        tools,
        [
            ("mail", "delete_mail"),
            ("mail", "delete_mail"),
            ("mail", "label"),
            ("vault", "open"),
            ("mail", "send_mail"),
            ("mail", "read_mail"),
            ("nowhere", "x")
        ]
    );
    let canonical = r#"{"a":[1,{"y":1,"z":0}],"b":2}"#;
    assert_eq!(
        rows[5]["args_sha"],
        crate::rooms::sealed::sha_hex(canonical.as_bytes()),
        "the sha of the canonical arguments"
    );
    assert_eq!(rows[0]["args_sha"], rows[1]["args_sha"]);
    let result = json!({"content": [{"type": "text", "text": "done"}]});
    assert_eq!(
        rows[5]["result_sha"],
        super::canon::sha(&result),
        "the sha of what ran answered"
    );
    for (i, r) in rows.iter().enumerate() {
        assert_eq!(
            (r["bot"].as_str(), r["conv"].as_str()),
            (Some("mail"), Some("bot-mail"))
        );
        assert!(!r["at"].as_str().unwrap().is_empty() && r["turn"].is_i64());
        assert_eq!(
            r["result_sha"].as_str().unwrap().is_empty(),
            i != 5,
            "a result only of what ran"
        );
        let kept = r.to_string();
        assert!(
            !kept.contains("done") && !kept.contains("a@b.c"),
            "never the content: {kept}"
        );
    }
}
