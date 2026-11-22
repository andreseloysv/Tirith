//! What the Access page shows of the connectors: the gateway's recorded calls, newest first and narrowed by bot and connector, and each connector's block state, for all bots and for each bot granted or blocked.
use super::kept;
use crate::app::App;
use serde_json::{json, Value};

const ROWS: usize = 300;

/// The recorded calls, newest first: time, bot, `connector.tool` and verdict; `bot` and `connector`, when not empty, narrow them.
pub fn calls(app: &App, bot: &str, connector: &str) -> Value {
    let sql = format!("SELECT at, bot, connector, tool, verdict FROM connector_calls WHERE (?1 = '' OR bot = ?1) AND (?2 = '' OR connector = ?2) ORDER BY id DESC LIMIT {ROWS}");
    let cols = ["at", "bot", "connector", "tool", "verdict"];
    let mut rows = app.objects(&sql, [bot, connector], &cols);
    for r in &mut rows {
        r["call"] = format!(
            "{}.{}",
            r["connector"].as_str().unwrap_or(""),
            r["tool"].as_str().unwrap_or("")
        )
        .into();
    }
    json!(rows)
}

/// The page id of the bot whose role is `key`.
fn id_of(bots: &[Value], key: &str) -> String {
    bots.iter()
        .find(|b| b["key"] == key)
        .and_then(|b| b["id"].as_str())
        .unwrap_or_else(|| key.strip_prefix("bot-").unwrap_or(key))
        .to_string()
}

/// Each connector: its id, whether it is blocked for all bots, and the bots it is granted to or blocked for (`blocked` then).
pub fn blocks(app: &App) -> Value {
    let cfg = app.cfg();
    let bots = crate::bots::all(app);
    let held = kept::all(app);
    let roles = cfg.factory.raw.get("roles").and_then(|r| r.as_table());
    let granted = |id: &str| -> Vec<String> {
        let roles = roles.into_iter().flatten();
        let has = |r: &toml::Value| r.get("connectors").and_then(|c| c.get(id)).is_some();
        roles
            .filter(|(k, r)| k.starts_with("bot-") && has(r))
            .map(|(k, _)| id_of(&bots, k))
            .collect()
    };
    let out: Vec<Value> = cfg
        .factory
        .connectors
        .iter()
        .map(|c| {
            let mut per: Vec<Value> = granted(&c.id)
                .into_iter()
                .map(|b| json!({"bot": b, "blocked": false}))
                .collect();
            let blocked = held.iter().filter(|(_, id)| *id == c.id);
            per.extend(blocked.map(|(b, _)| json!({"bot": b, "blocked": true})));
            json!({"id": c.id, "blocked": c.blocked, "bots": per})
        })
        .collect();
    json!(out)
}
