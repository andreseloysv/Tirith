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
    let missing: Vec<&str> = c
        .secrets
        .iter()
        .filter(|n| !names.contains(n))
        .map(String::as_str)
        .collect();
    if !missing.is_empty() {
        let why = format!("{} not set for this bot", missing.join(", "));
        return json!({"status": "needs login", "why": why});
    }
    let opened = state.is_some_and(|s| s["ok"] == true);
    let signed = crate::oauth::signed_in(names, &c.id);
    if !c.url.is_empty() && c.secrets.is_empty() && !signed && !opened {
        return json!({"status": "needs login", "why": "not signed in"});
    }
    json!({"status": "ok", "why": ""})
}
