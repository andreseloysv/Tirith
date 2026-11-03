//! The small feeds the state copies out of append-only logs: `violations`, `loose_events`, `says`,
//! `manager_queue` / `steward_queue` (`hooks.py`: read_hook_rows / attach / public, `collector._violation`,
//! `inbox.says`, `asks.queue_items`).

use crate::pytext;
use crate::store::read_jsonl;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub const VIOLATIONS_KEPT: usize = 50;
pub const LOOSE_EVENTS: usize = 50;
pub const SAY_ROWS: usize = 100;
pub const QUEUE_STATE_ROWS: usize = 50;
const READ_MAX_BYTES: u64 = 1_000_000;

/// Python truthiness of a JSON value.
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// `str(row.get(key) or "")`.
fn str_or_empty(row: &Value, key: &str) -> String {
    match row.get(key) {
        Some(v) if truthy(v) => match v {
            Value::String(s) => s.clone(),
            Value::Bool(_) => "True".to_string(),
            other => other.to_string(),
        },
        _ => String::new(),
    }
}

/// `str(value)` of a lane id (a string in practice).
pub fn id_str(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "None".to_string(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        other => other.to_string(),
    }
}

/// `row.get(key)`: the value, or null when the key is missing.
fn field(row: &Value, key: &str) -> Value {
    row.get(key).cloned().unwrap_or(Value::Null)
}

/// Rows of `kind` from the last megabyte of `events.jsonl` (the first line is cut when the file is longer).
pub fn read_kind_rows(path: &Path, kind: &str) -> Vec<Value> {
    let read = || -> std::io::Result<(u64, Vec<u8>)> {
        let mut file = fs::File::open(path)?;
        let size = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(size.saturating_sub(READ_MAX_BYTES)))?;
        let mut raw = Vec::new();
        file.read_to_end(&mut raw)?;
        Ok((size, raw))
    };
    let Ok((size, raw)) = read() else { return Vec::new() };
    let text = String::from_utf8_lossy(&raw);
    let mut lines = pytext::splitlines(&text);
    if size > READ_MAX_BYTES && !lines.is_empty() {
        lines.remove(0);
    }
    lines
        .into_iter()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|row| row.is_object() && row.get("kind").and_then(Value::as_str) == Some(kind))
        .collect()
}

pub fn read_hook_rows(home: &Path) -> Vec<Value> {
    read_kind_rows(&home.join("events.jsonl"), "hook")
}

pub fn read_violation_rows(home: &Path) -> Vec<Value> {
    read_kind_rows(&home.join("events.jsonl"), "violation")
}

/// Hook rows that belong to no known lane, as they appear in the state (last `LOOSE_EVENTS`).
pub fn loose_events(hook_rows: &[Value], lane_ids: &[String]) -> Vec<Value> {
    let loose: Vec<Value> = hook_rows
        .iter()
        .filter(|row| !row.get("lane").and_then(Value::as_str).is_some_and(|l| lane_ids.iter().any(|id| id == l)))
        .map(|row| {
            let mut out = Map::new();
            for key in ["ts", "agent", "event", "lane", "session", "project", "tool", "summary", "agent_session_id"] {
                out.insert(key.to_string(), field(row, key));
            }
            Value::Object(out)
        })
        .collect();
    loose[loose.len().saturating_sub(LOOSE_EVENTS)..].to_vec()
}

/// One violation row as the Rules section shows it: what was blocked and what to do instead.
pub fn violation(row: &Value) -> Value {
    let message = str_or_empty(row, "message");
    let (blocked, instead) = match message.split_once(". Do this instead: ") {
        Some((b, i)) => (b.to_string(), i.to_string()),
        None => (message.clone(), String::new()),
    };
    let prefix = format!("tirith: {}: ", str_or_empty(row, "rule"));
    let blocked = blocked.strip_prefix(&prefix).map(str::to_string).unwrap_or(blocked);
    json!({
        "ts": field(row, "ts"), "rule": str_or_empty(row, "rule"), "agent": field(row, "agent"),
        "lane": field(row, "lane"), "project": field(row, "project"), "message": message,
        "blocked": blocked, "instead": instead,
    })
}

pub fn violations(home: &Path) -> Vec<Value> {
    let rows = read_violation_rows(home);
    rows[rows.len().saturating_sub(VIOLATIONS_KEPT)..].iter().map(violation).collect()
}

/// The manager's plain messages to the owner (`say` rows of the inbox), oldest first, the last `SAY_ROWS`.
pub fn says(home: &Path) -> Vec<Value> {
    let out: Vec<Value> = read_jsonl(&home.join("inbox.jsonl"))
        .iter()
        .filter(|r| r.get("kind").and_then(Value::as_str) == Some("say"))
        .filter(|r| r.get("id").is_some_and(truthy) && r.get("text").is_some_and(truthy))
        .map(|r| {
            json!({"id": str_or_empty(r, "id"), "at": field(r, "at"), "project": str_or_empty(r, "project"),
                   "text": str_or_empty(r, "text"), "from": "manager"})
        })
        .collect();
    out[out.len().saturating_sub(SAY_ROWS)..].to_vec()
}

/// The manager's queue folded by item id (`asks.queue_items`); the old file name is read while the new
/// one does not exist.
pub fn queue_items(home: &Path, only_open: bool) -> Vec<Value> {
    let (new, old) = (home.join("manager_queue.jsonl"), home.join("steward_queue.jsonl"));
    let path = if old.exists() && !new.exists() { old } else { new };
    let mut order: Vec<String> = Vec::new();
    let mut items: HashMap<String, Map<String, Value>> = HashMap::new();
    for row in read_jsonl(&path) {
        let rid = str_or_empty(&row, "id");
        if rid.is_empty() {
            continue;
        }
        if row.get("done").is_some_and(truthy) {
            if let Some(item) = items.get_mut(&rid) {
                item.insert("status".into(), json!("done"));
                if row.get("result").is_some_and(truthy) {
                    item.insert("result".into(), json!(str_or_empty(&row, "result")));
                }
            }
        } else if !items.contains_key(&rid) {
            let kind = if row.get("kind").is_some_and(truthy) { field(&row, "kind") } else { json!("review") };
            let mut item = Map::new();
            item.insert("id".into(), json!(rid));
            item.insert("at".into(), field(&row, "at"));
            item.insert("kind".into(), kind);
            item.insert("project".into(), field(&row, "project"));
            item.insert("lane".into(), field(&row, "lane"));
            item.insert("text".into(), json!(str_or_empty(&row, "text")));
            item.insert("to".into(), json!("manager"));
            item.insert("status".into(), json!("open"));
            order.push(rid.clone());
            items.insert(rid, item);
        }
    }
    order
        .into_iter()
        .filter_map(|id| items.remove(&id))
        .filter(|i| !only_open || i.get("status").and_then(Value::as_str) == Some("open"))
        .map(Value::Object)
        .collect()
}

/// `manager_queue` in the state: open items, the last `QUEUE_STATE_ROWS`. Not wired into the state yet: Python
/// also adds items derived from `waiting_on_you`, so emitting only the file-backed ones would not match.
pub fn manager_queue(home: &Path) -> Vec<Value> {
    let rows = queue_items(home, true);
    rows[rows.len().saturating_sub(QUEUE_STATE_ROWS)..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn lines(path: &Path, rows: &[Value]) {
        let mut f = fs::File::create(path).unwrap();
        for r in rows {
            writeln!(f, "{r}").unwrap();
        }
    }

    #[test]
    fn violations_split_the_message() {
        let v = violation(&json!({"rule": "r1", "message": "tirith: r1: no raw git. Do this instead: tirith run"}));
        assert_eq!((v["blocked"].as_str(), v["instead"].as_str()), (Some("no raw git"), Some("tirith run")));
        let v = violation(&json!({"message": "plain"}));
        assert_eq!((v["blocked"].as_str(), v["rule"].as_str(), v["ts"].is_null()), (Some("plain"), Some(""), true));
    }

    #[test]
    fn loose_events_skip_known_lanes_and_keep_the_last_fifty() {
        let home = tempfile::tempdir().unwrap();
        let mut rows = vec![json!({"kind": "hook", "lane": "a", "event": "x"}), json!({"kind": "violation"})];
        rows.extend((0..60).map(|i| json!({"kind": "hook", "lane": "z", "event": i})));
        lines(&home.path().join("events.jsonl"), &rows);
        let hooks = read_hook_rows(home.path());
        assert_eq!(hooks.len(), 61);
        let loose = loose_events(&hooks, &["a".to_string()]);
        assert_eq!((loose.len(), loose[0]["event"].clone(), loose[0]["tool"].is_null()), (50, json!(10), true));
        assert_eq!(read_violation_rows(home.path()).len(), 1);
    }

    #[test]
    fn queue_folds_done_rows_and_says_need_id_and_text() {
        let home = tempfile::tempdir().unwrap();
        lines(
            &home.path().join("steward_queue.jsonl"),
            &[
                json!({"id": "q1", "text": "a"}),
                json!({"id": "q2", "kind": "k"}),
                json!({"id": "q1", "done": true, "result": "ok"}),
                json!({"done": true}),
            ],
        );
        let all = queue_items(home.path(), false);
        assert_eq!((all.len(), all[0]["status"].clone(), all[0]["result"].clone()), (2, json!("done"), json!("ok")));
        assert_eq!(manager_queue(home.path()).len(), 1);
        lines(
            &home.path().join("inbox.jsonl"),
            &[json!({"kind": "say", "id": 7, "text": "hi"}), json!({"kind": "say", "id": "x"}), json!({"kind": "ask", "id": "y", "text": "t"})],
        );
        assert_eq!(says(home.path()), vec![json!({"id": "7", "at": null, "project": "", "text": "hi", "from": "manager"})]);
    }
}
