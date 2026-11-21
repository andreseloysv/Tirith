//! When a routine runs: its days of the week, and whether it is due now.
use crate::server::s;
use chrono::{DateTime, Datelike, Local, NaiveTime};
use serde_json::{json, Value};
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
/// Due once a day: it is on, today is one of its days, its time has come and it has not run since then. A daemon that was down at that time runs it when it is back the same day.
pub fn due(r: &Value, now: DateTime<Local>) -> bool {
    let Ok(at) = NaiveTime::parse_from_str(&s(r, "at"), "%H:%M") else {
        return false;
    };
    let today = now.date_naive().and_time(at);
    let last = DateTime::parse_from_rfc3339(&s(r, "last"))
        .ok()
        .map(|t| t.with_timezone(&Local).naive_local());
    r["on"] != false
        && r["days"]
            .as_array()
            .is_some_and(|d| d.contains(&json!(now.weekday().number_from_monday())))
        && now.naive_local() >= today
        && last.is_none_or(|l| l < today)
}
