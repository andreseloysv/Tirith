//! The experiments' rows: one read, the running ones, and the list the page and `tirith lab list` show.
use crate::app::App;
use rusqlite::params;
use serde_json::{json, Value};
use std::path::PathBuf;

/// One experiment, as its row holds it.
#[derive(Clone)]
pub struct Experiment {
    pub id: i64,
    pub bot: String,
    pub routine: String,
    pub branch: String,
    pub created_at: String,
    pub mode: String,
}

const COLS: &str = "id, bot, routine, branch, created_at, mode";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Experiment> {
    Ok(Experiment {
        id: r.get(0)?,
        bot: r.get(1)?,
        routine: r.get(2)?,
        branch: r.get(3)?,
        created_at: r.get(4)?,
        mode: r.get(5)?,
    })
}

/// Experiment `id`, while it runs.
pub fn running_one(app: &App, id: i64) -> Option<Experiment> {
    let sql = format!("SELECT {COLS} FROM experiments WHERE id = ?1 AND status = 'running'");
    app.query(&sql, [id], row).into_iter().next()
}

/// The experiment (live or replay) running on routine `routine` of the bot whose conversation is `bot`.
pub fn running(app: &App, bot: &str, routine: &str) -> Option<Experiment> {
    let sql = format!(
        "SELECT {COLS} FROM experiments WHERE bot = ?1 AND routine = ?2 AND status = 'running'"
    );
    let rows = app.query(&sql, params![bot, routine], row);
    rows.into_iter().next()
}

/// Every experiment still running.
pub fn all_running(app: &App) -> Vec<Experiment> {
    let sql = format!("SELECT {COLS} FROM experiments WHERE status = 'running'");
    app.query(&sql, [], row)
}

/// The worktree a branch runs in: `<home>/lab/<bot>/<experiment>`.
pub fn tree(app: &App, e: &Experiment) -> PathBuf {
    app.home.join("lab").join(&e.bot).join(e.id.to_string())
}

const LIST: &str = "SELECT id, bot, routine, branch, change, hypothesis, mode, status, created_at, COALESCE(ended_at, ''), COALESCE(numbers, '{}'), (SELECT COUNT(*) FROM lab_comparisons c WHERE c.experiment = e.id AND verdict IS NOT NULL) FROM experiments e WHERE ?1 = '' OR bot = ?1 ORDER BY id DESC LIMIT 50";

/// The bot's experiments (every bot's with `bot` empty), newest first: what changed, the hypothesis, the mode, the comparisons judged and, once decided, the numbers and the line said.
pub fn list(app: &App, bot: &str) -> Vec<Value> {
    app.query(LIST, [bot], |r| {
        let numbers: String = r.get(10)?;
        let t = |i: usize| r.get::<_, String>(i);
        Ok(json!({
            "id": r.get::<_, i64>(0)?, "bot": t(1)?, "routine": t(2)?, "branch": t(3)?,
            "change": t(4)?, "hypothesis": t(5)?, "mode": t(6)?, "status": t(7)?,
            "created_at": t(8)?, "ended_at": t(9)?,
            "numbers": serde_json::from_str::<Value>(&numbers).unwrap_or_default(),
            "compared": r.get::<_, i64>(11)?,
        }))
    })
}
