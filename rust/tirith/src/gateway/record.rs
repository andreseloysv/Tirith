//! Every attempt through a gateway, as a row of `connector_calls` beside the bot's turns: when, the bot, its conversation and the turn open then, the connector and the tool, the sha256 of the canonical arguments and of the result, and the verdict (an approval call's: waiting for approval, or approved and ran). Never the arguments or the result themselves.
use super::{canon::sha, session::Session};
use crate::app::{now_iso, Shared};
use serde_json::Value;

/// Records one attempt with its verdict `word`; `result` is what ran answered (None when nothing ran).
pub fn record(
    app: &Shared,
    s: &Session,
    (connector, tool): (&str, &str),
    args: &Value,
    result: Option<&Value>,
    word: &str,
) {
    let turn = crate::turns::open_start(&s.key);
    let result = result.map(sha).unwrap_or_default();
    let sql = "INSERT INTO connector_calls (at, bot, conv, turn, connector, tool, args_sha, result_sha, verdict) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)";
    let row = rusqlite::params![
        now_iso(),
        s.bot,
        s.key,
        turn,
        connector,
        tool,
        sha(args),
        result,
        word
    ];
    let _ = app.db.lock().unwrap().execute(sql, row);
}
