//! What the conversation is told when a routine's schedule changes: nobody waits for it on a day, or at a time, it no longer has.
use crate::server::s;
use chrono::{DateTime, Local};
use serde_json::Value;

/// The schedule of a routine as far as it decides when it runs.
fn schedule(r: &Value) -> (String, Vec<u64>, String, bool) {
    let mut days: Vec<u64> = r["days"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_u64)
        .collect();
    days.sort_unstable();
    days.dedup();
    let timed = s(r, "trigger") != "webhook";
    let trigger = if timed { "schedule" } else { "webhook" };
    (trigger.into(), days, s(r, "at"), r["on"] != false)
}

/// What the line of routine `r`, which was `was`, adds; none when its schedule is the same.
pub fn changed(was: &Value, r: &Value, now: DateTime<Local>) -> Option<String> {
    let ((before, ..), (trigger, _, _, on)) = (schedule(was), schedule(r));
    let timed = trigger == "schedule";
    if schedule(was) == schedule(r) || (!timed && before != "schedule") {
        return None;
    }
    Some(match (timed, on) {
        (false, _) => " · no schedule now: it waits for its event".into(),
        (true, false) => format!(
            "{} · turned off: it will not run",
            super::words::days_and_time(r)
        ),
        (true, true) => super::words::runs(r, now, "next time"),
    })
}
