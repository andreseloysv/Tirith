//! The issue key of a bot's turn: what its cost is attributed to (`neobot costs --by issue`). In order: the key the run set with `neobot turn key ABC-123`, a leading `[ABC-123]` in its final answer, the key its task or event named (leading, or in brackets).
use super::live::with;
use super::signals::last;
use crate::app::Shared;
use crate::client::{call_json, Flag};
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use rusqlite::params;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Default)]
struct Keys {
    /// What the next message named, what the open turn's did, and what the run itself said.
    hint: String,
    current: String,
    said: String,
}
static KEYS: Mutex<BTreeMap<String, Keys>> = Mutex::new(BTreeMap::new());
fn keys<T>(conv: &str, f: impl FnOnce(&mut Keys) -> T) -> T {
    f(KEYS.lock().unwrap().entry(conv.to_string()).or_default())
}

/// The message about to be handed to `conv` names this key (or none).
pub fn hint(conv: &str, text: &str) {
    keys(conv, |k| k.hint = crate::costs::found(text));
}
/// A turn of `conv` opens: it has the key of the message that starts it.
pub(super) fn opened(conv: &str) {
    keys(conv, |k| {
        (k.current, k.said) = (std::mem::take(&mut k.hint), String::new())
    });
}
/// The key of the turn of `conv` that ends now with `answer`.
pub(super) fn take(conv: &str, answer: &str) -> String {
    let (said, current) = keys(conv, |k| {
        (std::mem::take(&mut k.said), std::mem::take(&mut k.current))
    });
    [said, crate::costs::issue_key(answer.trim_start()), current]
        .into_iter()
        .find(|k| !k.is_empty())
        .unwrap_or_default()
}

/// `neobot turn key <ABC-123>`, on the conversation of the session that runs it.
pub fn cli(pos: &[&str], flag: Flag) -> Option<Result<(), String>> {
    let ["turn", "key", key] = pos else {
        return None;
    };
    let conv = flag("--project").or_else(|| std::env::var("TIRITH_PROJECT").ok());
    let body = match (crate::costs::issue_key(key) == *key, conv) {
        (false, _) => Err(format!("'{key}' is no issue key, as ABC-123")),
        (_, None) => Err("no conversation: run it in a bot's session".into()),
        (_, Some(conv)) => Ok(json!({"conv": conv, "key": key})),
    };
    let sent = body.and_then(|b| call_json("POST", "/api/turns/key", Some(b)));
    Some(sent.map(|v| {
        println!(
            "issue {} kept on this turn",
            v["key"].as_str().unwrap_or("")
        )
    }))
}

/// Keeps `key` on the open turn of `conv`, else on its last kept turn (and the judge's turns of it).
pub fn keep(app: &Shared, conv: &str, key: &str) -> Result<(), String> {
    let open = with(conv, |c| c.open.is_some());
    if open {
        keys(conv, |k| k.said = key.to_string());
        return Ok(());
    }
    let id = last(app, conv).ok_or(format!("no turn in '{conv}' yet"))?;
    app.insert(
        "UPDATE turns SET issue = ?1 WHERE id = ?2",
        params![key, id],
    );
    Ok(())
}

async fn post_key(
    State(app): State<Shared>,
    who: crate::server::Who,
    Json(b): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let conv = app.conv(b["conv"].as_str().unwrap_or("")).to_string();
    let key = b["key"].as_str().unwrap_or("");
    let own = who.agent().is_none_or(|a| app.conv(&a.project) == conv);
    let said = match (own, crate::costs::issue_key(key) == key && !key.is_empty()) {
        (false, _) => Err("a bot keeps the key on its own conversation's turns".to_string()),
        (_, false) => Err("an issue key, as ABC-123".into()),
        _ => keep(&app, &conv, key),
    };
    match said {
        Ok(()) => (StatusCode::OK, Json(json!({"conv": conv, "key": key}))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e}))),
    }
}
pub fn routes() -> Router<Shared> {
    Router::new().route("/api/turns/key", post(post_key))
}
