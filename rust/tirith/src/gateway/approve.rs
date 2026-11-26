//! A call whose verdict asks the owner first: it runs only on a permit the owner's yes granted for exactly this call (connector, tool, canonical arguments), once; otherwise it does not run, an approval card names the bot, the connector, the tool and a secret-free summary of the arguments, and the bot is told to end its turn.
use super::permit::{ask, call_sha, consume};
use super::{given::secrets, link::Link, rpc::failed, session::Session, summary::summary};
use crate::app::Shared;
use serde_json::{json, Value};

/// What the record says of one that ran on the owner's yes.
pub const APPROVED: &str = "approved and ran";
/// What it says of one that waits for the owner.
pub const WAITING: &str = "waiting for approval";

/// The result of approval call `tool` of `connector` with `args`, the record's word, and whether it ran.
pub async fn approval(
    app: &Shared,
    s: &Session,
    (connector, tool): (&str, &str),
    args: &Value,
    link: Option<&Link>,
) -> (Value, &'static str, bool) {
    let call = call_sha(connector, tool, args);
    let Some(card) = consume(app, &s.key, &call) else {
        return (
            asked(app, s, (connector, tool), args, &call).await,
            WAITING,
            false,
        );
    };
    let params = json!({"name": tool, "arguments": args});
    let mut result = match link {
        Some(l) => l
            .request("tools/call", params)
            .await
            .unwrap_or_else(|e| failed(&e)),
        None => failed(&format!(
            "connector '{connector}' is not open in this gateway"
        )),
    };
    if let Some(r) = result.as_object_mut() {
        let meta = r.entry("_meta").or_insert_with(|| json!({}));
        let _ = meta.as_object_mut().map(|m| {
            m.insert(
                "tirith/approval".into(),
                json!({"verdict": APPROVED, "card": card}),
            )
        });
    }
    let said = format!("{connector}.{tool}: {APPROVED} on the owner's yes (card {card})");
    app.event("", &s.key, "gateway", APPROVED, &said, "");
    (result, APPROVED, true)
}

/// The card asking for this call (one already open, else a new one), and what the bot is told.
async fn asked(
    app: &Shared,
    s: &Session,
    (connector, tool): (&str, &str),
    args: &Value,
    call: &str,
) -> Value {
    let cfg = app.cfg();
    let names = cfg.factory.connectors.iter().find(|c| c.id == connector);
    let names = names.map(|c| c.secrets.clone()).unwrap_or_default();
    let values = secrets(app, &s.bot, (&names, &[]))
        .await
        .unwrap_or_default();
    let values: Vec<String> = values.into_iter().map(|(_, v)| v).collect();
    let question = format!(
        "Bot «{}» asks to run tool {tool} of connector {connector} with: {}. Answer yes to let this exact call run once, or no.",
        s.bot,
        summary(args, &values)
    );
    drop(values);
    match ask(app, &s.key, call, &question) {
        Ok(card) => failed(&format!(
            "{connector}.{tool}: approval required; it did not run. Card {card} asks the owner. End your turn now: their yes lets this exact call run once in a later turn; any other arguments, a second run or a no ask again"
        )),
        Err(e) => failed(&format!(
            "{connector}.{tool}: approval required; it did not run, and no card could ask the owner: {e}"
        )),
    }
}
