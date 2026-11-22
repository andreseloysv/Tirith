//! The quality signals on a turn's row: the owner's 👍/👎 (`reaction`), a correction after it (`corrected`), its own validation (`checks_passed`, `checks_total`, `checks`) and the numbered notes its answer cites (`cited`).
use super::record::Turn;
use crate::app::Shared;
use rusqlite::{params, Connection};
use serde_json::json;
use std::sync::LazyLock;

/// The signal columns, added to an older `turns` table (each ALTER fails once its column exists).
pub fn add_columns(db: &Connection) {
    [
        "reaction INTEGER",
        "corrected INTEGER NOT NULL DEFAULT 0",
        "checks_passed INTEGER",
        "checks_total INTEGER",
        "checks TEXT",
        "cited TEXT",
        "score REAL",
        "experiment INTEGER",
        "variant TEXT",
        "why TEXT",
    ]
    .into_iter()
    .for_each(|col| drop(db.execute(&format!("ALTER TABLE turns ADD COLUMN {col}"), [])));
}

static NOTE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\[([ms][0-9]+)\]").unwrap());

/// The numbered notes (`[m<N>]` its own memory's, `[s<N>]` the shared one's) an answer names, each once, in their order.
pub fn cited(answer: &str) -> Vec<String> {
    let mut notes: Vec<String> = vec![];
    for c in NOTE.captures_iter(answer) {
        let _ = (!notes.contains(&c[1].to_string())).then(|| notes.push(c[1].to_string()));
    }
    notes
}

/// What a closing turn adds to its row `id`: the notes its answer cites and the checks it reported while open.
pub fn keep(app: &Shared, id: i64, t: &Turn, answer: &str) {
    let cited = json!(cited(answer)).to_string();
    let c = t.checks.as_ref();
    app.insert(
        "UPDATE turns SET cited = ?1, checks_passed = ?2, checks_total = ?3, checks = ?4 WHERE id = ?5",
        params![cited, c.map(|c| c.0), c.map(|c| c.1), c.map(|c| &c.2), id],
    );
}

/// The last turn of conversation `conv` already kept: its own role's (the judge's turns on it are kept there too).
pub fn last(app: &Shared, conv: &str) -> Option<i64> {
    let role = app.chat_role(conv);
    let sql = "SELECT id FROM turns WHERE conv = ?1 AND role = ?2 ORDER BY id DESC LIMIT 1";
    app.query(sql, params![conv, role], |r| r.get(0))
        .into_iter()
        .next()
}

/// An eval's label on an owner message: `owner-mood`'s `correction` or `repeat` marks `before`, the conversation's last turn that ended before that message, as corrected.
pub fn judged(app: &Shared, before: Option<i64>, eval: &str, label: &str) {
    let against = eval == "owner-mood" && matches!(label, "correction" | "repeat");
    let Some(id) = before.filter(|_| against) else {
        return;
    };
    app.insert("UPDATE turns SET corrected = 1 WHERE id = ?1", params![id]);
    app.emit("turn", json!({"id": id, "corrected": 1}));
}
