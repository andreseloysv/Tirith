// Connectors on the Access page (src/access/): the gateway's calls as the owner reads them, and Block / Unblock for one bot and for all, against the gateway checks' fake stdio and HTTP MCP servers (sandbox = false, no model).
use super::tests::{call, hello, http, text, world, World, BOT};
use serde_json::{json, Value};

fn owner(w: &World) -> String {
    crate::server::testing::owner(w.port)
}

/// The owner's GET of `path`, as JSON.
async fn get(w: &World, path: &str) -> Value {
    let (code, body) = http(w.port, &owner(w), "GET", path, Value::Null).await;
    assert_eq!(code, 200, "{path}: {body}");
    serde_json::from_str(&body).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn connector_calls_show_the_bot() {
    let w = world("access-calls").await;
    let mut msgs = hello();
    msgs.push(call(3, "mail__read_mail", json!({"q": "x"})));
    msgs.push(call(4, "web__fetch", json!({})));
    msgs.push(call(5, "mail__delete_mail", json!({})));
    let out = w.gateway(BOT, "mail", &msgs).await.unwrap();
    assert_eq!(text(&out[3]), "fetched", "{out:?}");

    let rows = get(&w, "/api/connectors/calls").await;
    let rows = rows.as_array().unwrap();
    let seen: Vec<(&str, &str, &str)> = rows
        .iter()
        .map(|r| {
            let s = |k: &str| r[k].as_str().unwrap();
            (s("bot"), s("call"), s("verdict"))
        })
        .collect();
    assert_eq!(
        seen,
        [
            ("mail", "mail.delete_mail", "refused by deny"),
            ("mail", "web.fetch", "ran"),
            ("mail", "mail.read_mail", "ran")
        ],
        "newest first, as time, bot, connector.tool and verdict"
    );
    assert!(rows.iter().all(|r| !r["at"].as_str().unwrap().is_empty()));
    let n = |v: Value| v.as_array().unwrap().len();
    assert_eq!(n(get(&w, "/api/connectors/calls?connector=web").await), 1);
    assert_eq!(
        n(get(&w, "/api/connectors/calls?bot=mail&connector=mail").await),
        2
    );
    assert_eq!(n(get(&w, "/api/connectors/calls?bot=other").await), 0);
    for path in ["/api/connectors/calls", "/api/connectors/blocks"] {
        let (code, _) = http(w.port, &w.token(BOT), "GET", path, Value::Null).await;
        assert_eq!(code, 403, "{path} is the owner's alone");
    }

    // The remote's proxy rows are the bot's, never the daemon's or the manager role it runs on.
    let net: Vec<String> = w.app.query(
        "SELECT DISTINCT lane FROM events WHERE kind = 'net'",
        [],
        |r| r.get(0),
    );
    assert_eq!(net, ["bot-mail"]);
    let dests = get(&w, "/api/net/destinations").await;
    let local = dests
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["host"] == "127.0.0.1")
        .expect("the remote's destination");
    let lanes: Vec<&str> = local["lanes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap())
        .collect();
    assert!(lanes.contains(&"bot-mail"), "{lanes:?}");
}

/// The session id of a gateway opened by the bot's run.
async fn opened(w: &World) -> String {
    let (code, body) = http(
        w.port,
        &w.token(BOT),
        "POST",
        "/api/mcp/open",
        json!({"bot": "mail"}),
    )
    .await;
    assert_eq!(code, 200, "{body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    v["session"].as_str().unwrap().to_string()
}

/// One call through the open gateway `session`: what the agent reads.
async fn through(w: &World, session: &str, name: &str, q: &str) -> String {
    let path = format!("/api/mcp/{session}/call");
    let body = json!({"name": name, "arguments": {"q": q}});
    let (_, said) = http(w.port, &w.token(BOT), "POST", &path, body).await;
    let result: Value = serde_json::from_str(&said).unwrap_or_else(|_| panic!("{said}"));
    text(&json!({ "result": result }))
}

/// The owner's Block (`on`) or Unblock: its status and body.
async fn block(w: &World, body: Value, on: bool) -> (u16, String) {
    let method = if on { "POST" } else { "DELETE" };
    http(w.port, &owner(w), method, "/api/connectors/blocks", body).await
}

fn config(w: &World) -> String {
    std::fs::read_to_string(w.app.home.join("config.toml")).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn blocked_connector_refuses_next_call() {
    let w = world("access-block").await;
    let file = w.app.home.join("config.toml");
    let first = format!("# the owner's own note\n{}", config(&w));
    let first = first.replace(
        "[roles.bot-mail.connectors.web]",
        "# web for the mail bot\n[roles.bot-mail.connectors.web]",
    );
    std::fs::write(&file, &first).unwrap();
    assert!(w.app.reload_config(true));
    let session = opened(&w).await;
    assert!(through(&w, &session, "mail__read_mail", "one")
        .await
        .starts_with("done"));

    // One bot: its grant table goes, every comment stays, and the open gateway refuses the next call.
    let (code, said) = block(&w, json!({"connector": "mail", "bot": "mail"}), true).await;
    assert_eq!(code, 200, "{said}");
    let now = config(&w);
    assert!(!now.contains("[roles.bot-mail.connectors.mail]"), "{now}");
    assert!(now.contains("# the owner's own note") && now.contains("# web for the mail bot"));
    assert!(
        now.contains("[roles.bot-mail.connectors.web]")
            && now.contains("[roles.bot-mail.connectors.vault]")
    );
    let said = through(&w, &session, "mail__read_mail", "two").await;
    assert!(
        said.contains("refused by block") && !w.calls().contains("two"),
        "{said}"
    );
    assert!(
        through(&w, &session, "web__fetch", "w")
            .await
            .starts_with("fetched"),
        "the bot's other grants stand"
    );
    let blocks = get(&w, "/api/connectors/blocks").await;
    let mail = blocks
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "mail")
        .unwrap()
        .clone();
    assert_eq!(
        mail["bots"],
        json!([{"bot": "mail", "blocked": true}]),
        "{blocks}"
    );
    let (code, _) = block(&w, json!({"connector": "mail", "bot": "mail"}), true).await;
    assert_eq!(code, 400, "a grant that is gone is not taken twice");

    // Its Unblock lays the very table back where it was.
    let (code, said) = block(&w, json!({"connector": "mail", "bot": "mail"}), false).await;
    assert_eq!(code, 200, "{said}");
    assert_eq!(config(&w), first, "the exact file again");
    assert!(through(&w, &session, "mail__read_mail", "three")
        .await
        .starts_with("done"));
    let (code, _) = block(&w, json!({"connector": "mail", "bot": "mail"}), false).await;
    assert_eq!(code, 400, "nothing is blocked to unblock");

    // All bots: blocked = true on the connector's row, its grants untouched.
    let (code, said) = block(&w, json!({"connector": "web"}), true).await;
    assert_eq!(code, 200, "{said}");
    let now = config(&w);
    assert!(
        now.contains("blocked = true") && now.contains("[roles.bot-mail.connectors.web]"),
        "{now}"
    );
    let said = through(&w, &session, "web__fetch", "w2").await;
    assert!(said.contains("refused by block"), "{said}");
    let blocks = get(&w, "/api/connectors/blocks").await;
    let web = blocks
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "web")
        .unwrap()
        .clone();
    assert_eq!(web["blocked"], true, "{blocks}");
    let (code, said) = block(&w, json!({"connector": "web"}), false).await;
    assert_eq!(code, 200, "{said}");
    assert_eq!(config(&w), first, "the exact file again");
    assert!(through(&w, &session, "web__fetch", "w3")
        .await
        .starts_with("fetched"));

    let verdicts: Vec<String> =
        w.app
            .query("SELECT verdict FROM connector_calls ORDER BY id", [], |r| {
                r.get(0)
            });
    let expected = [
        "ran",
        "refused by block",
        "ran",
        "ran",
        "refused by block",
        "ran",
    ];
    assert_eq!(verdicts, expected, "each attempt recorded with its verdict");
    let (code, _) = block(&w, json!({"connector": "nowhere"}), true).await;
    assert_eq!(code, 400, "an unknown connector");
    let (code, _) = http(
        w.port,
        &w.token(BOT),
        "POST",
        "/api/connectors/blocks",
        json!({"connector": "mail"}),
    )
    .await;
    assert_eq!(code, 403, "an agent blocks nothing");
}
