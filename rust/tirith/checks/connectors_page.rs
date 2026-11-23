// The Connectors page and a bot card's Connectors (src/connector_page/, src/bots/connect.rs): each bot's status, a grant given, narrowed and removed through the bot-card API, a connector removed with every grant, and no secret in any answer, against the gateway checks' fake MCP servers (no model, no sandbox).
use super::tests::{hello, http, world, World, BOT, MAIL_SECRET, SECRETS};
use serde_json::{json, Value};

/// Every answer the test read, to look for a secret in all of them at the end.
struct Seen<'a>(&'a World, Vec<String>);

impl Seen<'_> {
    async fn call(&mut self, method: &str, path: &str, body: Value) -> (u16, Value) {
        let owner = crate::server::testing::owner(self.0.port);
        let (code, said) = http(self.0.port, &owner, method, path, body).await;
        self.1.push(said.clone());
        (
            code,
            serde_json::from_str(&said).unwrap_or(Value::String(said)),
        )
    }
    async fn page(&mut self) -> Value {
        let (code, page) = self.call("GET", "/api/connectors/page", Value::Null).await;
        assert_eq!(code, 200, "{page}");
        page
    }
    async fn grant(&mut self, bot: &str, body: Value) -> (u16, Value) {
        self.call("POST", &format!("/api/bots/{bot}/connectors"), body)
            .await
    }
}

/// Connector `id` of the page, and its row for bot `bot` (Null when it has none).
fn row(page: &Value, id: &str, bot: &str) -> (Value, Value) {
    let list = page["connectors"].as_array().unwrap();
    let c = list
        .iter()
        .find(|c| c["id"] == id)
        .cloned()
        .unwrap_or(Value::Null);
    let bots = c["bots"].as_array().cloned().unwrap_or_default();
    let b = bots
        .into_iter()
        .find(|b| b["bot"] == bot)
        .unwrap_or(Value::Null);
    (c, b)
}

fn marks(b: &Value) -> Vec<(String, String)> {
    let tools = b["tools"].as_array().cloned().unwrap_or_default();
    let s = |t: &Value, k: &str| t[k].as_str().unwrap().to_string();
    tools.iter().map(|t| (s(t, "name"), s(t, "mark"))).collect()
}

fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

fn config(w: &World) -> String {
    std::fs::read_to_string(w.app.home.join("config.toml")).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn connectors_page_state() {
    let w = world("page").await;
    let file = w.app.home.join("config.toml");
    std::fs::write(&file, format!("# the owner's own note\n{}", config(&w))).unwrap();
    assert!(w.app.reload_config(true));
    w.gateway(BOT, "mail", &hello()).await.unwrap(); // opens mail and web: their real tools are seen
    let mut s = Seen(&w, vec![]);

    // Status aggregation: each granted bot, its status, its tools marked by the gateway's own verdict.
    let page = s.page().await;
    let (mail, of_mail) = row(&page, "mail", "mail");
    assert_eq!(
        (mail["kind"].as_str(), mail["host"].as_str()),
        (Some("local"), Some("")),
        "{mail}"
    );
    assert_eq!(
        mail["bots"].as_array().unwrap().len(),
        1,
        "only bot mail is granted mail: {mail}"
    );
    assert_eq!(of_mail["status"], "ok", "{of_mail}");
    let want = [
        ("read_mail", "allowed"),
        ("send_mail", "approval"),
        ("delete_mail", "denied"),
        ("label", "denied"),
    ];
    assert_eq!(marks(&of_mail), pairs(&want));
    let (web, of_web) = row(&page, "web", "mail");
    assert_eq!(
        (web["kind"].as_str(), of_web["status"].as_str()),
        (Some("remote"), Some("ok")),
        "{web}"
    );
    assert!(
        web["host"].as_str().unwrap().starts_with("127.0.0.1:")
            && !page.to_string().contains("/mcp")
    );
    assert!(
        !page.to_string().contains("gateway_fakes") && !page.to_string().contains("\"command\""),
        "no command: {page}"
    );
    let (vault, _) = row(&page, "vault", "mail");
    assert_eq!(vault["blocked"], true);
    assert!(
        row(&page, "web", "other").1.is_null(),
        "bot other is not granted web"
    );

    // A grant round-trip through the bot-card API: written under the bot's own role, kept by a card save.
    let (code, card) = s
        .grant("other", json!({"connector": "web", "grant": {}}))
        .await;
    assert_eq!(code, 200, "{card}");
    let web_card = card
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "web")
        .unwrap()
        .clone();
    assert_eq!(
        (web_card["granted"].as_bool(), web_card["tools"].is_null()),
        (Some(true), true),
        "{card}"
    );
    let now = config(&w);
    assert!(
        now.contains("[roles.bot-other.connectors.web]") && now.contains("# the owner's own note"),
        "{now}"
    );
    let connectors_rows = now.split("[roles").next().unwrap();
    assert!(
        !connectors_rows.contains("bots"),
        "never a bots list on the connector: {now}"
    );
    let (code, _) = s
        .call(
            "POST",
            "/api/bots",
            json!({"id": "other", "name": "Other", "color": "#123456"}),
        )
        .await;
    assert_eq!(code, 200);
    assert!(
        config(&w).contains("[roles.bot-other.connectors.web]"),
        "a card save keeps the grant"
    );
    let (_, got) = s
        .call("GET", "/api/bots/other/connectors", Value::Null)
        .await;
    assert_eq!(got, card, "the card reads what it wrote");
    let (_, of_other) = row(&s.page().await, "web", "other");
    assert_eq!(
        of_other["status"], "needs login",
        "its secrets are not set: {of_other}"
    );
    assert!(of_other["why"].as_str().unwrap().contains("WEB_KEY"));

    // Narrowing: tools, deny and approval on the grant; the page marks the tools by them.
    let narrow = json!({"tools": ["fetch", "post"], "deny": ["post"], "approval": ["fetch"]});
    let (code, _) = s
        .grant("other", json!({"connector": "web", "grant": narrow}))
        .await;
    assert_eq!(code, 200);
    assert!(
        config(&w).contains("tools = [\"fetch\", \"post\"]"),
        "{}",
        config(&w)
    );
    let (_, of_other) = row(&s.page().await, "web", "other");
    assert_eq!(
        marks(&of_other),
        pairs(&[("fetch", "approval"), ("post", "denied")])
    );
    let (code, _) = s
        .grant(
            "other",
            json!({"connector": "web", "grant": {"deny": [""]}}),
        )
        .await;
    assert_eq!(code, 400, "an empty glob is refused");
    let (code, _) = s
        .grant("other", json!({"connector": "nope", "grant": {}}))
        .await;
    assert_eq!(code, 400, "an unknown connector is refused");
    for (method, path) in [
        ("GET", "/api/connectors/page"),
        ("POST", "/api/bots/mail/connectors"),
        ("POST", "/api/connectors/web/remove"),
    ] {
        let (code, _) = http(
            w.port,
            &w.token(BOT),
            method,
            path,
            json!({"connector": "web", "grant": {}}),
        )
        .await;
        assert_eq!(code, 403, "{path} is the owner's alone");
    }

    // Removal of one grant: the other bot's grant of the same connector stands.
    let (code, _) = s
        .grant("other", json!({"connector": "web", "grant": null}))
        .await;
    assert_eq!(code, 200);
    let now = config(&w);
    assert!(
        !now.contains("bot-other.connectors") && now.contains("[roles.bot-mail.connectors.web]"),
        "{now}"
    );
    assert!(w.app.cfg().factory.granted("bot-other").is_empty());

    // An error from a login is shown, never a secret: the fake server has no OAuth, and a why that held one is scrubbed.
    let (code, said) = s
        .call("POST", "/api/connectors/web/login", json!({"bot": "mail"}))
        .await;
    assert_ne!(code, 200, "{said}");
    let (_, of_web) = row(&s.page().await, "web", "mail");
    assert_eq!(of_web["status"], "error", "{of_web}");
    let leak = format!(
        "401 for Bearer {MAIL_SECRET}; access_token={MAIL_SECRET}x at https://h/x?code=abc"
    );
    crate::connector_page::failed(&w.app, "mail", "mail", &leak).await;
    let (_, of_mail) = row(&s.page().await, "mail", "mail");
    let why = of_mail["why"].as_str().unwrap();
    assert!(
        of_mail["status"] == "error" && why.contains("401") && !why.contains("abc"),
        "{why}"
    );

    // Remove: the definition and every grant go, comments stay.
    let (code, said) = s
        .call("POST", "/api/connectors/vault/remove", Value::Null)
        .await;
    assert_eq!(code, 200, "{said}");
    let now = config(&w);
    assert!(
        !now.contains("\"vault\"")
            && !now.contains("connectors.vault")
            && now.contains("# the owner's own note"),
        "{now}"
    );
    assert!(row(&s.page().await, "vault", "mail").0.is_null());
    assert_eq!(
        w.app.cfg().factory.granted("bot-mail").len(),
        2,
        "mail and web stand"
    );
    let (code, _) = s
        .call("POST", "/api/connectors/vault/remove", Value::Null)
        .await;
    assert_eq!(code, 400, "a connector that is gone is not removed twice");

    // No secret value in any answer.
    for said in &s.1 {
        for secret in SECRETS {
            assert!(!said.contains(secret), "a secret leaked: {said}");
        }
    }
}
