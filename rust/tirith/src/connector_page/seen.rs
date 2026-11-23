//! What the daemon last saw of a connector: the tools its last open listed (kv `connector-tools:<id>`, names only) and, by bot, whether its last open or sign-in worked or why not (kv `connector-state:<bot>:<id>`, the why scrubbed of every secret).
use super::scrub::scrub;
use crate::app::{now_iso, App, Shared};
use serde_json::{json, Value};

fn tools_key(id: &str) -> String {
    format!("connector-tools:{id}")
}

fn state_key(bot: &str, id: &str) -> String {
    format!("connector-state:{bot}:{id}")
}

/// The tools connector `id` listed when it last opened, by name.
pub fn tools(app: &App, id: &str) -> Vec<String> {
    let text = app.kv(&tools_key(id)).unwrap_or_default();
    serde_json::from_str(&text).unwrap_or_default()
}

/// How bot `bot`'s last open or sign-in of `id` went: `{at, ok}` or `{at, error}`.
pub fn state(app: &App, bot: &str, id: &str) -> Option<Value> {
    serde_json::from_str(&app.kv(&state_key(bot, id))?).ok()
}

/// Connector `id` worked for bot `bot`; `names`, when given, are the tools it listed.
pub fn worked(app: &App, bot: &str, id: &str, names: Option<&[String]>) {
    if let Some(names) = names {
        app.set_kv(&tools_key(id), Some(&json!(names).to_string()));
    }
    let state = json!({"at": now_iso(), "ok": true}).to_string();
    app.set_kv(&state_key(bot, id), Some(&state));
}

/// Every value bot `bot` holds, read where the store blocks.
async fn held(app: &Shared, bot: &str) -> Vec<String> {
    let (app, bot) = (app.clone(), bot.to_string());
    let read = move || {
        let names = crate::secrets::names(&app, &bot).unwrap_or_default();
        let value = |n: &String| crate::secrets::value(&app, &bot, n).ok().flatten();
        names.iter().filter_map(value).collect::<Vec<String>>()
    };
    tokio::task::spawn_blocking(read).await.unwrap_or_default()
}

/// The open or sign-in of `id` for bot `bot` failed with `why`: kept with each of the bot's values and anything like a token taken out; that scrubbed why, the one to show anywhere.
pub async fn failed(app: &Shared, bot: &str, id: &str, why: &str) -> String {
    let why = scrub(why, &held(app, bot).await);
    let state = json!({"at": now_iso(), "error": why}).to_string();
    app.set_kv(&state_key(bot, id), Some(&state));
    app.emit("connectors", json!({}));
    why
}

/// Forgets all that was seen of connector `id`.
pub fn forget(app: &App, id: &str) {
    let sql = "SELECT key FROM kv WHERE key LIKE 'connector-state:%' OR key = ?1";
    let keys: Vec<String> = app.query(sql, [tools_key(id)], |r| r.get(0));
    let ours = |k: &&String| k.starts_with("connector-tools:") || k.ends_with(&format!(":{id}"));
    keys.iter().filter(ours).for_each(|k| app.set_kv(k, None));
}
