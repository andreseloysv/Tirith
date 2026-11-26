//! One bot's status with one connector: `error` (its last open or sign-in failed, with why), `needs login` (a secret its definition names is not set for the bot, or a remote one that names none is neither signed in to nor ever opened), or `ok`.
use super::seen;
use crate::app::App;
use crate::connectors::Connector;
use serde_json::{json, Value};

/// Bot `bot`'s status with `c`, `names` being the names (never the values) it holds.
pub fn status(app: &App, bot: &str, c: &Connector, names: &[String]) -> Value {
    let state = seen::state(app, bot, &c.id);
    if let Some(why) = state.as_ref().and_then(|s| s["error"].as_str()) {
        return json!({"status": "error", "why": why});
    }
    let lacks = |only_required: bool| -> Vec<&str> {
        let first = c.secrets.first().filter(|_| c.command.is_empty());
        let held = |n: &&String| !names.contains(n);
        let must = |n: &&String| !only_required || c.required.contains(n) || first == Some(*n);
        c.secrets
            .iter()
            .filter(held)
            .filter(must)
            .map(String::as_str)
            .collect()
    };
    if !lacks(true).is_empty() {
        let why = format!(
            "{} is required and not set for this bot",
            lacks(true).join(", ")
        );
        return json!({"status": "needs login", "why": why});
    }
    let optional = lacks(false);
    let opened = state.is_some_and(|s| s["ok"] == true);
    let signed = crate::oauth::signed_in(names, &c.id);
    if !c.url.is_empty() && c.secrets.is_empty() && !signed && !opened {
        return json!({"status": "needs login", "why": "not signed in"});
    }
    let why = match optional.is_empty() {
        true => String::new(),
        false => format!(
            "opens without {}: not set for this bot, and not required",
            optional.join(", ")
        ),
    };
    json!({"status": "ok", "why": why})
}
