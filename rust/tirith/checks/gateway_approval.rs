// A gateway call that asks the owner first (src/gateway/approve.rs, permit.rs): the fake stdio MCP server of the gateway checks, the test daemon with sandbox = false, no model. Self-approval here is a bot's caller token sent to the daemon's own routes; no sandbox is started or probed.
use super::tests::{call, hello, http, text, world, World, BOT, MAIL_SECRET};
use serde_json::{json, Value};

/// How many tools/call of `tool` the fake server got.
fn ran(w: &World, tool: &str) -> usize {
    w.calls().lines().filter(|l| l.contains(tool)).count()
}

/// The card a refused call's text names.
fn card_of(said: &str) -> String {
    let at = said
        .find("Card o")
        .unwrap_or_else(|| panic!("no card: {said}"));
    said[at + 5..]
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}

/// One call through a fresh gateway of the bot: its reply.
async fn once(w: &World, name: &str, args: Value) -> Value {
    let mut msgs = hello();
    msgs.push(call(3, name, args));
    let out = w.gateway(BOT, "mail", &msgs).await.unwrap();
    out[2].clone()
}

fn card(w: &World, id: &str) -> (String, String, String, String) {
    let sql = "SELECT kind, project, question, status FROM owner_items WHERE id = ?1";
    let rows = w.app.query(sql, [id], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
    });
    rows.into_iter().next().expect("the card")
}

fn permit(w: &World, id: &str) -> String {
    let sql = "SELECT status FROM connector_permits WHERE card = ?1";
    let rows = w.app.query(sql, [id], |r| r.get::<_, String>(0));
    rows.into_iter().next().unwrap_or_default()
}

/// The owner (or `token`) answers card `id`: the status code.
async fn answer(w: &World, token: &str, id: &str, body: Value) -> u16 {
    http(w.port, token, "POST", &format!("/api/owner/{id}"), body)
        .await
        .0
}

fn verdicts(w: &World) -> Vec<String> {
    let rows = w.rows();
    let word = |r: &Value| r["verdict"].as_str().unwrap().to_string();
    rows.iter().map(word).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn approval_is_bound_to_the_call() {
    let w = world("approval").await;
    let owner = crate::server::testing::owner(w.port);
    let bot = w.token(BOT);
    let args = json!({"to": "ana@example.com", "subject": "hola", "api_key": MAIL_SECRET, "body": format!("token {MAIL_SECRET}")});

    // No run before yes: a card naming the bot, the connector and the tool, with no secret.
    let first = once(&w, "mail__send_mail", args.clone()).await;
    let said = text(&first);
    assert!(
        first["result"]["isError"] == true && said.contains("approval required"),
        "{first}"
    );
    assert!(
        said.contains("End your turn"),
        "the bot is told to stop: {said}"
    );
    let o1 = card_of(&said);
    let (kind, project, question, status) = card(&w, &o1);
    assert_eq!(
        (kind.as_str(), project.as_str(), status.as_str()),
        ("approval", "bot-mail", "open")
    );
    for word in [
        "«mail»",
        "send_mail",
        "connector mail",
        "ana@example.com",
        "hola",
    ] {
        assert!(question.contains(word), "{word} in: {question}");
    }
    assert!(!question.contains(MAIL_SECRET), "a secret in: {question}");
    assert_eq!(ran(&w, "send_mail"), 0, "nothing ran before yes");
    assert_eq!(permit(&w, &o1), "asked");
    let again = once(&w, "mail__send_mail", args.clone()).await;
    assert_eq!(
        card_of(&text(&again)),
        o1,
        "asking again opens no second card"
    );

    // A caller inside the bot's run cannot approve its own call.
    let forged = json!({"answer": "yes", "by": "owner"});
    assert_eq!(
        answer(&w, &bot, &o1, forged.clone()).await,
        400,
        "a forged card answer"
    );
    let words = json!({"at": "2999-01-01T00:00:00Z", "text": "yes"});
    w.app
        .set_kv("owner_said:bot-mail", Some(&words.to_string()));
    assert_eq!(
        answer(&w, &bot, &o1, forged.clone()).await,
        400,
        "nor in the owner's words"
    );
    w.app.set_kv("owner_said:bot-mail", None);
    assert_ne!(
        answer(&w, "", &o1, forged.clone()).await,
        200,
        "nor without a token"
    );
    let (_, opened) = http(
        w.port,
        &bot,
        "POST",
        "/api/mcp/open",
        json!({"bot": "mail", "role": "owner"}),
    )
    .await;
    let session = serde_json::from_str::<Value>(&opened).unwrap()["session"]
        .as_str()
        .unwrap()
        .to_string();
    let body = json!({"name": "mail__send_mail", "arguments": args, "approved": true, "card": o1, "by": "owner", "verdict": "ran"});
    let (_, said) = http(
        w.port,
        &bot,
        "POST",
        &format!("/api/mcp/{session}/call"),
        body,
    )
    .await;
    assert!(
        said.contains("approval required"),
        "a forged gateway request: {said}"
    );
    assert_eq!(
        (card(&w, &o1).3.as_str(), permit(&w, &o1).as_str()),
        ("open", "asked")
    );
    assert_eq!(ran(&w, "send_mail"), 0, "no self-approval ran anything");

    // The owner's yes: a permit for exactly this call.
    assert_eq!(answer(&w, &owner, &o1, json!({"answer": "yes"})).await, 200);
    assert_eq!(permit(&w, &o1), "granted");

    // A changed argument, or another tool, asks again and runs nothing.
    let mut changed = args.clone();
    changed["subject"] = "hola!".into();
    let other = once(&w, "mail__send_mail", changed).await;
    assert_ne!(card_of(&text(&other)), o1, "changed arguments ask again");
    let fax = once(&w, "mail__send_fax", args.clone()).await;
    assert_ne!(card_of(&text(&fax)), o1, "another tool asks again");
    assert_eq!(ran(&w, "send_"), 0);
    assert_eq!(permit(&w, &o1), "granted");

    // The identical call runs exactly once, said apart from an ordinary run.
    let reordered = format!(
        r#"{{"body":"token {MAIL_SECRET}","api_key":"{MAIL_SECRET}","subject":"hola","to":"ana@example.com"}}"#
    );
    let reordered: Value = serde_json::from_str(&reordered).unwrap(); // the same arguments in another order
    let yes = once(&w, "mail__send_mail", reordered).await;
    assert_eq!(text(&yes), "done", "{yes}");
    assert_eq!(
        yes["result"]["_meta"]["tirith/approval"],
        json!({"verdict": "approved and ran", "card": o1})
    );
    assert_eq!(ran(&w, "send_mail"), 1);
    assert_eq!(permit(&w, &o1), "used");

    // No reuse: a second identical run needs a new card.
    let twice = once(&w, "mail__send_mail", args.clone()).await;
    let o4 = card_of(&text(&twice));
    assert_ne!(o4, o1);
    assert_eq!(ran(&w, "send_mail"), 1, "the permit ran one call");

    // A no creates no permit.
    assert_eq!(
        answer(&w, &owner, &o4, json!({"answer": "no, not today"})).await,
        200
    );
    assert_eq!(permit(&w, &o4), "refused");
    let after_no = once(&w, "mail__send_mail", args.clone()).await;
    assert!(text(&after_no).contains("approval required"));
    assert_eq!(ran(&w, "send_mail"), 1, "a no runs nothing");

    let v = verdicts(&w);
    assert_eq!(
        v.iter().filter(|v| *v == "approved and ran").count(),
        1,
        "{v:?}"
    );
    assert!(
        v.iter()
            .all(|v| v == "waiting for approval" || v == "approved and ran"),
        "{v:?}"
    );
    let rows = w.rows();
    let done = rows
        .iter()
        .find(|r| r["verdict"] == "approved and ran")
        .unwrap();
    assert!(!done["result_sha"].as_str().unwrap().is_empty());
    let kept = serde_json::to_string(&rows).unwrap();
    assert!(!kept.contains(MAIL_SECRET) && !kept.contains("ana@example.com"));
}
