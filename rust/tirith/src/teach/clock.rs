//! The extension's clock and the daemon's may differ by minutes or hours: a step's time is rebuilt on the daemon's clock from how long before sending it happened, and the extension is told how long a recording has left, not when it ends.
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
fn parse(step: &Value, key: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(step[key].as_str()?)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}
/// When the step happened on the daemon's clock, as `now` (the daemon's time of arrival, written as `state::now_ms`) minus the time between the step (`at`) and its sending (`sent`), both the extension's; just `now` when it says only one of them or an odd span.
pub(super) fn at(step: &Value, now: &str) -> String {
    let arrived = DateTime::parse_from_rfc3339(now).map(|t| t.with_timezone(&Utc));
    let age = parse(step, "sent")
        .zip(parse(step, "at"))
        .map(|(sent, at)| sent - at)
        .filter(|a| *a >= Duration::zero() && *a <= Duration::minutes(10));
    match (arrived, age) {
        (Ok(now), Some(age)) => (now - age).format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        _ => now.to_string(),
    }
}
