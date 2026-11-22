//! A bot's memory in numbers: what loading it costs in tokens, how often its last 30 runs cited each numbered note, the bot's own notes none cited once 30 runs have happened since they were written (candidates to delete; Tirith never deletes them, and a shared note is never proposed from one bot's runs) and the memory misses (owner messages owner-mood labelled `repeat`).
use super::rows::Row;
use crate::app::Shared;
use serde_json::{json, Value};
use std::sync::LazyLock;

/// The runs whose citations count.
const LAST: usize = 30;

static NOTE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?m)^[ \t]*[-*] \[([ms][0-9]+)\][ \t]*(.*?)[ \t]*$").unwrap()
});

/// The numbered notes of a memory's text whose tag is `tag`: (`m3`, its words).
pub fn notes(text: &str, tag: char) -> Vec<(String, String)> {
    NOTE.captures_iter(text)
        .filter(|c| c[1].starts_with(tag))
        .map(|c| (c[1].to_string(), c[2].to_string()))
        .collect()
}

/// The owner messages in conversation `key` that owner-mood labelled `repeat`, the last first.
fn misses(app: &Shared, key: &str) -> Vec<String> {
    app.query("SELECT events.summary FROM scores JOIN events ON events.id = scores.event WHERE scores.eval = 'owner-mood' AND scores.label = 'repeat' AND events.project = ?1 ORDER BY scores.id DESC", [key], |r| r.get::<_, String>(0))
}

/// The memory part of the numbers: `own` and `shared` are the texts of the bot's memory.md and of the shared one.
pub fn memory(app: &Shared, key: &str, rows: &[Row], (own, shared): (&str, &str)) -> Value {
    let last = &rows[rows.len().saturating_sub(LAST)..];
    let cites = |id: &str| {
        last.iter()
            .filter(|r| r.cited.iter().any(|c| c == id))
            .count()
    };
    let mut all = notes(shared, 's');
    all.extend(notes(own, 'm'));
    let listed: Vec<Value> = all
        .iter()
        .map(|(id, text)| json!({"id": id, "text": text, "cited": cites(id)}))
        .collect();
    let dir = app.home.join("manager").join(key);
    let old = |id: &str| super::age::runs_since(app, key, &dir, id).is_some_and(|n| n >= LAST);
    let uncited: Vec<&String> = all
        .iter()
        .filter(|(id, _)| id.starts_with('m') && cites(id) == 0 && old(id))
        .map(|n| &n.0)
        .collect();
    let missed = misses(app, key);
    let tokens = |t: &str| t.chars().count() / 4;
    json!({
        "tokens": {"own": tokens(own), "shared": tokens(shared)},
        "runs": last.len(),
        "notes": listed,
        "uncited": uncited,
        "misses": missed.len(),
        "missed": missed.iter().take(3).collect::<Vec<_>>(),
    })
}
