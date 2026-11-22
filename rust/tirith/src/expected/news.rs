//! The news of a good run wait for its judge, so a "could not" that it finds is the owner's one line and not a second one after the answer. They wait as long as a judge can take and no longer, and they wait in the store, so no restart loses them.
use super::unjudged::WAITS;
use crate::app::{now_iso, Shared};
use chrono::{DateTime, Local, TimeDelta};
use serde_json::{json, Value};

const KEY: &str = "news:run:";
/// How much longer than a judge's turns they wait: code's plain words look first at a run that no judge read (`unjudged.rs`).
const MARGIN: TimeDelta = TimeDelta::minutes(1);

/// `text`, what the run of turn `turn` in `conv` has for the phones, waits.
pub(super) fn keep(app: &Shared, conv: &str, turn: i64, text: &str) {
    let kept = json!({"conv": conv, "text": text, "at": now_iso()});
    app.set_kv(&format!("{KEY}{turn}"), Some(&kept.to_string()));
}

/// What waits under `key`, for who takes it first.
fn take(app: &Shared, key: &str) -> Option<Value> {
    let sql = "DELETE FROM kv WHERE key = ?1 RETURNING value";
    let kept: Vec<String> = app.query(sql, [key], |r| r.get(0));
    serde_json::from_str(kept.first()?).ok()
}

/// The run of turn `turn` failed: its one line takes their place.
pub(super) fn drop_them(app: &Shared, turn: i64) {
    take(app, &format!("{KEY}{turn}"));
}

/// The run of turn `turn` is judged, or will not be: its news go to the phones. True when some waited.
pub fn settled(app: &Shared, turn: i64) -> bool {
    out(app, &format!("{KEY}{turn}"))
}

fn out(app: &Shared, key: &str) -> bool {
    let Some(kept) = take(app, key) else {
        return false;
    };
    let part = |name: &str| kept[name].as_str().unwrap_or("").to_string();
    crate::rooms::to_phones(app, &part("conv"), &part("text"));
    true
}

/// Those that no judge's word will come for any more go out.
pub(super) fn overdue(app: &Shared, now: DateTime<Local>) {
    let sql = "SELECT key, value FROM kv WHERE key LIKE ?1";
    let like = format!("{KEY}%");
    let all: Vec<(String, String)> = app.query(sql, [&like], |r| Ok((r.get(0)?, r.get(1)?)));
    for (key, value) in all {
        let kept: Value = serde_json::from_str(&value).unwrap_or_default();
        let at = kept["at"]
            .as_str()
            .and_then(|t| DateTime::parse_from_rfc3339(t).ok());
        if at.is_none_or(|at| now.signed_duration_since(at) > WAITS + MARGIN) {
            out(app, &key);
        }
    }
}
