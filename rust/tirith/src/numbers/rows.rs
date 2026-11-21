//! A bot's own turns as the numbers read them, the first first.
use crate::app::Shared;
use rusqlite::Row as SqlRow;

/// One run of a bot: its routine (empty in a chat turn), how it ended, its time, cost, tokens and quality signals.
#[derive(Clone, Debug, Default)]
pub struct Row {
    pub id: i64,
    pub routine: String,
    pub ok: bool,
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
}

/// A JSON list of names, empty when missing.
fn names(text: Option<String>) -> Vec<String> {
    text.and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn row(r: &SqlRow) -> rusqlite::Result<Row> {
    let (passed, total) = (r.get::<_, Option<i64>>(10)?, r.get::<_, Option<i64>>(11)?);
    Ok(Row {
        id: r.get(0)?,
        routine: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
        ok: r.get::<_, Option<String>>(2)?.as_deref() == Some("ok"),
        ms: r.get::<_, Option<i64>>(3)?.unwrap_or(0),
        cost: r.get(4)?,
        tokens: r.get::<_, Option<i64>>(5)?.unwrap_or(0),
        reaction: r.get(6)?,
        corrected: r.get::<_, Option<i64>>(7)?.unwrap_or(0) == 1,
        score: r.get(8)?,
        skills: names(r.get(9)?),
        validated: total.is_some() && passed == total,
        cited: names(r.get(12)?),
    })
}

/// The turns of the bot whose conversation is `key`, under its own role (the judge's turns on it are kept there too), the first first.
pub fn rows(app: &Shared, key: &str) -> Vec<Row> {
    let sql = "SELECT id, routine, status, ms, cost_usd, COALESCE(tokens_in, 0) + COALESCE(tokens_out, 0) + COALESCE(tokens_cache_read, 0) + COALESCE(tokens_cache_write, 0), reaction, corrected, score, skills, checks_passed, checks_total, cited FROM turns WHERE conv = ?1 AND role = ?1 ORDER BY id";
    app.query(sql, [key], row)
}
