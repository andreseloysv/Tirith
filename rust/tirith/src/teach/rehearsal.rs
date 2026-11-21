//! A bot's rehearsals of a draft in the owner's Edge, kept in kv (`rehearse:<bot>`, the last few, newest last): which skill, since when, whether it is on, and each step with its result and screenshot.
use super::state::{now_ms, MOST_S};
use crate::app::{now_s, App};
use serde_json::{json, Value};
use std::sync::Mutex;
/// At most this many rehearsals are kept per bot.
const KEPT: usize = 5;
static WRITING: Mutex<()> = Mutex::new(());
pub(super) fn all(app: &App, key: &str) -> Vec<Value> {
    app.kv(&format!("rehearse:{key}"))
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default()
}
/// Changes the rehearsals under one lock, and says what `f` returned.
pub(super) fn change<T>(app: &App, key: &str, f: impl FnOnce(&mut Vec<Value>) -> T) -> T {
    let _held = WRITING.lock().unwrap_or_else(|e| e.into_inner());
    let mut list = all(app, key);
    let out = f(&mut list);
    let from = list.len().saturating_sub(KEPT);
    let kept = Value::from(list.split_off(from));
    app.set_kv(&format!("rehearse:{key}"), Some(&kept.to_string()));
    out
}
/// Whether the rehearsal is on and still inside its 10 minutes.
pub(super) fn live(r: &Value) -> bool {
    r["on"] == true && now_s() < until_s(r)
}
/// When the rehearsal stops by itself, in seconds since the epoch.
pub(super) fn until_s(r: &Value) -> u64 {
    r["since_s"].as_u64().unwrap_or(0) + MOST_S
}
/// The newest rehearsal, if any.
pub(super) fn last(app: &App, key: &str) -> Option<Value> {
    all(app, key).pop()
}
/// Starts a rehearsal of `skill` (the one before stops): the new one.
pub(super) fn begin(app: &App, key: &str, skill: &str) -> Value {
    let r = json!({"on": true, "since": now_ms(), "since_s": now_s(), "skill": skill, "steps": []});
    change(app, key, |list| {
        list.iter_mut().for_each(|o| o["on"] = json!(false));
        list.push(r.clone());
    });
    r
}
/// Stops the newest rehearsal; whether one was on.
pub(super) fn end(app: &App, key: &str) -> bool {
    change(app, key, |list| {
        list.last_mut().is_some_and(|r| {
            let was = r["on"] == true;
            r["on"] = json!(false);
            was
        })
    })
}
/// Adds a step to the newest rehearsal: its number.
pub(super) fn push(app: &App, key: &str, mut step: Value) -> usize {
    change(app, key, |list| {
        let Some(steps) = list.last_mut().and_then(|r| r["steps"].as_array_mut()) else {
            return 0;
        };
        step["n"] = json!(steps.len() + 1);
        steps.push(step);
        steps.len()
    })
}
