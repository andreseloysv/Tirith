//! The captures on disk: `<home>/captures/<turn>.jsonl`, its first line what started the turn and then one line per tool call; those of the last 10 runs of each routine of a bot are kept (its turns outside a routine count as one more), the rest go.
use super::record::Turn;
use crate::app::App;
use crate::server::s;
use rusqlite::params;
use serde_json::{json, Value};
use std::path::PathBuf;

/// How many runs of each routine keep their capture.
const KEEP: usize = 10;

/// One capture read back: its turn, what started it and its calls in order.
#[derive(Clone)]
pub struct Captured {
    pub turn: i64,
    pub start: String,
    pub calls: Vec<Value>,
}

fn file(app: &App, turn: i64) -> PathBuf {
    app.home.join("captures").join(format!("{turn}.jsonl"))
}

/// Turn `id` of a bot ended: its capture is written, and the oldest of its routine beyond the last 10 go.
pub fn keep(app: &App, id: i64, t: &Turn) {
    let c = &t.capture;
    let head = json!({"turn": id, "conv": t.conv, "routine": t.routine, "trigger": t.trigger, "start": c.start, "full": c.full});
    let lines: Vec<String> = std::iter::once(&head)
        .chain(&c.calls)
        .map(Value::to_string)
        .collect();
    let path = file(app, id);
    let _ = std::fs::create_dir_all(path.parent().unwrap_or(&path));
    let _ = std::fs::write(&path, lines.join("\n") + "\n");
    let sql = "SELECT id FROM turns WHERE conv = ?1 AND role = ?1 AND routine = ?2 ORDER BY id DESC LIMIT -1 OFFSET ?3";
    let old = app.query(sql, params![t.conv, t.routine, KEEP as i64], |r| {
        r.get::<_, i64>(0)
    });
    old.into_iter()
        .for_each(|o| drop(std::fs::remove_file(file(app, o))));
}

/// The capture of turn `turn`, when it is still kept.
pub fn capture_of(app: &App, turn: i64) -> Option<Captured> {
    let text = std::fs::read_to_string(file(app, turn)).ok()?;
    let mut lines = text
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok());
    let head = lines.next()?;
    Some(Captured {
        turn,
        start: s(&head, "start"),
        calls: lines.collect(),
    })
}

/// The last `n` runs of routine `routine` of the bot `conv` that kept their capture, oldest first.
pub fn captured(app: &App, conv: &str, routine: &str, n: usize) -> Vec<i64> {
    let sql =
        "SELECT id FROM turns WHERE conv = ?1 AND role = ?1 AND routine = ?2 ORDER BY id DESC";
    let ids = app.query(sql, params![conv, routine], |r| r.get::<_, i64>(0));
    let mut kept: Vec<i64> = ids
        .into_iter()
        .filter(|id| file(app, *id).is_file())
        .take(n)
        .collect();
    kept.reverse();
    kept
}
