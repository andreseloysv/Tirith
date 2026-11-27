//! When a routine runs: its days of the week, whether it is due now, and how late.
use crate::server::s;
use chrono::{DateTime, Datelike, Days, Local, NaiveDateTime, NaiveTime, TimeDelta};
use serde_json::{json, Value};
/// How long after its time a run is still on time: the ticks that look come every 30 to 60 s.
const ON_TIME: TimeDelta = TimeDelta::minutes(5);
/// Weekdays 1 (Monday) to 7, from `[1, 3]`, "daily", "weekdays", "weekends" or names such as "mon,wed".
pub fn days(v: &Value) -> Option<Vec<u64>> {
    let t = v.as_str().map(str::to_lowercase);
    let named = |d: &u64| {
        t.as_deref().is_some_and(|t| {
            t == "daily"
                || (t == "weekdays" && *d < 6)
                || (t == "weekends" && *d > 5)
                || t.contains(["mon", "tue", "wed", "thu", "fri", "sat", "sun"][*d as usize - 1])
        })
    };
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(Value::as_u64)
                .filter(|d| (1..=7).contains(d))
                .collect()
        })
        .or_else(|| t.is_some().then(|| (1..=7).filter(named).collect()))
}
/// The first instant the clock shows `t`: of the two in autumn, `earliest` can be the second.
fn first(t: NaiveDateTime) -> Option<DateTime<Local>> {
    let shown = t.and_local_timezone(Local);
    shown.earliest().min(shown.latest())
}
/// The latest of its times, on one of its days, that has come and that it has not run since: one, however many were missed. One that never ran has only today's.
fn pending(r: &Value, now: DateTime<Local>) -> Option<DateTime<Local>> {
    let at = NaiveTime::parse_from_str(&s(r, "at"), "%H:%M").ok()?;
    let on = r["days"].as_array()?;
    let today = now.date_naive();
    let time = (0..=7)
        .filter_map(|back| today.checked_sub_days(Days::new(back)))
        .filter(|day| on.contains(&json!(day.weekday().number_from_monday())))
        .filter_map(|day| {
            let t = day.and_time(at);
            // a time the clock jumps over in spring is the one an hour later
            first(t).or_else(|| first(t + TimeDelta::hours(1)))
        })
        .find(|time| *time <= now)?;
    let new = DateTime::parse_from_rfc3339(&s(r, "last"))
        .map_or(time.date_naive() == today, |last| last < time);
    new.then_some(time)
}
/// Due when it is on and one of its times is pending. Times missed (the daemon was down) are one run with `catch_up` "once", the default; with "skip" it runs only on time, so it waits for the next.
pub fn due(r: &Value, now: DateTime<Local>) -> bool {
    owed(r, now).is_some()
}
/// The one of its times it is to run for now, so the one that is not lost (`expected/missed.rs`).
pub fn owed(r: &Value, now: DateTime<Local>) -> Option<DateTime<Local>> {
    (r["on"] != false).then_some(())?;
    if !s(r, "every").is_empty() {
        return super::interval::owed(r, now); // every so often: no day, no time, no catch_up
    }
    pending(r, now).filter(|time| r["catch_up"] != "skip" || !past(*time, now))
}
/// Whether `time` is too long ago for a run of it to be on time.
pub fn past(time: DateTime<Local>, now: DateTime<Local>) -> bool {
    now - time > ON_TIME
}
/// What a run says of its delay, empty when it is on time.
pub(super) fn late(r: &Value, now: DateTime<Local>) -> String {
    let Some(time) = pending(r, now).filter(|time| past(*time, now)) else {
        return String::new();
    };
    let by = now - time;
    let (d, h, m) = (by.num_days(), by.num_hours() % 24, by.num_minutes() % 60);
    let by = match (d, h) {
        (0, 0) => format!("{m} min"),
        (0, _) => format!("{h} h {m} min"),
        _ => format!("{d} d {h} h"),
    };
    format!(
        ", {by} late (it was for {})",
        time.format("%a %Y-%m-%d %H:%M")
    )
}
