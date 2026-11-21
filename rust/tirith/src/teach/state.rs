//! A bot's recording, kept in kv (`teach:<bot>`): whether it is on, since when, its steps and the owner's words; and the bot's token (`teach-token:<bot>`).
use crate::app::{now_s, App};
use serde_json::{json, Value};
use std::sync::Mutex;
/// A recording stops by itself after this long.
pub(super) const MOST_S: u64 = 600;
/// At most this many steps are kept in one recording.
const MOST_STEPS: usize = 2000;
static WRITING: Mutex<()> = Mutex::new(());
/// The time now with milliseconds, as the extension writes it (so steps and words sort by time).
pub(super) fn now_ms() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}
pub(super) fn read(app: &App, key: &str) -> Value {
    app.kv(&format!("teach:{key}"))
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_else(|| json!({"on": false, "steps": []}))
}
/// Changes the recording under one lock, and says what `f` returned.
pub(super) fn change<T>(app: &App, key: &str, f: impl FnOnce(&mut Value) -> T) -> T {
    let _held = WRITING.lock().unwrap_or_else(|e| e.into_inner());
    let mut rec = read(app, key);
    let out = f(&mut rec);
    app.set_kv(&format!("teach:{key}"), Some(&rec.to_string()));
    out
}
/// Whether the recording is on and still inside its 10 minutes.
pub(super) fn live(rec: &Value) -> bool {
    rec["on"] == true && now_s() < rec["since_s"].as_u64().unwrap_or(0) + MOST_S
}
/// Adds a step or the owner's words to a live recording; false when none is on.
pub(super) fn push(app: &App, key: &str, entry: Value) -> bool {
    change(app, key, |rec| {
        let Some(steps) = live(rec).then(|| rec["steps"].as_array_mut()).flatten() else {
            return false;
        };
        let room = steps.len() < MOST_STEPS;
        let _ = room.then(|| steps.push(entry));
        room
    })
}
/// The bot's token: the one the extension sends, made once.
pub(super) fn token(app: &App, key: &str) -> String {
    let name = format!("teach-token:{key}");
    app.kv(&name).unwrap_or_else(|| {
        let t: String = (0..4)
            .map(|_| format!("{:08x}", rand::random::<u32>()))
            .collect();
        app.set_kv(&name, Some(&t));
        t
    })
}
