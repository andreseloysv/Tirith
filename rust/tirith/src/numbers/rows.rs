//! A bot's own turns as the numbers read them, the first first.
use crate::app::Shared;
use rusqlite::Row as SqlRow;

/// One run of a bot: its routine (empty in a chat turn), how it ended, its time, cost, tokens and quality signals.
#[derive(Clone, Debug, Default)]
pub struct Row {
    pub id: i64,
    pub routine: String,
    pub ok: bool,
    /// It ended with an approval card still open: neither a failure nor a good run until the owner answers.
    pub waiting: bool,
    pub ms: i64,
    pub cost: Option<f64>,
    /// tokens_in + tokens_out + the cache's read and written tokens.
    pub tokens: i64,
    pub reaction: Option<i64>,
    pub corrected: bool,
    pub score: Option<f64>,
    pub skills: Vec<String>,
    /// Its own validation passed: `checks_passed = checks_total`.
    pub validated: bool,
    pub cited: Vec<String>,
    /// It never became a turn (`expected/lost.rs`): a failure with no time and no tokens.
    pub never: bool,
    /// Why it failed, and when it ended.
    pub why: String,
    pub at: String,
}

/// A JSON list of names, empty when missing.
fn names(text: Option<String>) -> Vec<String> {
    text.and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn row(r: &SqlRow) -> rusqlite::Result<Row> {
    let status = r.get::<_, Option<String>>(2)?;
    let (passed, total) = (r.get::<_, Option<i64>>(10)?, r.get::<_, Option<i64>>(11)?);
    Ok(Row {
        id: r.get(0)?,
        routine: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
        ok: status.as_deref() == Some("ok"),
        waiting: status.as_deref() == Some("waiting"),
        ms: r.get::<_, Option<i64>>(3)?.unwrap_or(0),
        cost: r.get(4)?,
        tokens: r.get::<_, Option<i64>>(5)?.unwrap_or(0),
        reaction: r.get(6)?,
        corrected: r.get::<_, Option<i64>>(7)?.unwrap_or(0) == 1,
        score: r.get(8)?,
        skills: names(r.get(9)?),
        validated: total.is_some() && passed == total,
        cited: names(r.get(12)?),
        never: r.get(13)?,
        why: r.get(14)?,
        at: r.get(15)?,
    })
}

/// The turns of the bot whose conversation is `key`, under its own role (the judge's turns on it are kept there too), the first first. The B runs of experiments are not its own runs, and a replay's or a shadow's run is in a conversation of its own.
pub fn rows(app: &Shared, key: &str) -> Vec<Row> {
    let sql = "SELECT id, routine, status, ms, cost_usd, COALESCE(tokens_in, 0) + COALESCE(tokens_out, 0) + COALESCE(tokens_cache_read, 0) + COALESCE(tokens_cache_write, 0), reaction, corrected, score, skills, checks_passed, checks_total, cited, COALESCE(status = 'failed', 0) AND ms IS NULL AND COALESCE(agent, '') = '', COALESCE(why, ''), COALESCE(ended_at, '') FROM turns WHERE conv = ?1 AND role = ?1 AND COALESCE(variant, '') <> 'B' ORDER BY id";
    app.query(sql, [key], row)
}
