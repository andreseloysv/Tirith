//! The grants a one-bot block took out, kept in the store (`connector-block:<bot>:<connector>`) until its unblock lays them back.
use crate::app::App;
use serde_json::Value;

const PREFIX: &str = "connector-block:";

fn name(bot: &str, id: &str) -> String {
    format!("{PREFIX}{bot}:{id}")
}

/// What bot `bot`'s block of connector `id` took out, when it is blocked.
pub fn kept(app: &App, bot: &str, id: &str) -> Option<Value> {
    serde_json::from_str(&app.kv(&name(bot, id))?).ok()
}

/// Keeps (or, with None, forgets) what bot `bot`'s block of `id` took out.
pub fn keep(app: &App, bot: &str, id: &str, grant: Option<&Value>) {
    let text = grant.map(Value::to_string);
    app.set_kv(&name(bot, id), text.as_deref());
}

/// Whether the owner blocked connector `id` for bot `bot` (its page id).
pub fn blocked_for(app: &App, bot: &str, id: &str) -> bool {
    app.kv(&name(bot, id)).is_some()
}

/// Every one-bot block: (bot, connector).
pub fn all(app: &App) -> Vec<(String, String)> {
    let sql = "SELECT key FROM kv WHERE key LIKE 'connector-block:%' ORDER BY key";
    let keys = app.query(sql, [], |r| r.get::<_, String>(0));
    keys.iter()
        .filter_map(|k| k.strip_prefix(PREFIX)?.rsplit_once(':'))
        .map(|(b, c)| (b.to_string(), c.to_string()))
        .collect()
}
