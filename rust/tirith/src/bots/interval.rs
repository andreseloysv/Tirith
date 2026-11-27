//! A routine that runs every so often (`every = "2m"`): its words, its least, and when it is next due.
use crate::server::s;
use chrono::{DateTime, Local, TimeDelta};
use serde_json::{json, Value};

/// The shortest interval, the ticks that start a run come every 30 to 60 s; and the longest.
const LEAST: i64 = 60;
const MOST: i64 = 7 * 86400;

/// The seconds of `3m`, `90s`, `2h` or `1d` (a number and one unit), within the least and the most.
pub fn secs(words: &str) -> Result<i64, String> {
    let w = words.trim().to_lowercase();
    let unit = w.chars().last().and_then(|u| "smhd".find(u));
    let n: Option<i64> = w
        .get(..w.len().saturating_sub(1))
        .and_then(|n| n.parse().ok());
    let secs = unit.zip(n).map(|(u, n)| n * [1, 60, 3600, 86400][u]);
    secs.filter(|s| (LEAST..=MOST).contains(s)).ok_or(format!(
        "every as a number and s, m, h or d, from 1m to 7d, as 2m (not '{words}')"
    ))
}

/// The words of `secs`, the largest unit that holds it whole: 120 is `2m`.
pub fn words(secs: i64) -> String {
    let (n, u) = [(86400, "d"), (3600, "h"), (60, "m")]
        .into_iter()
        .find(|(d, _)| secs % d == 0)
        .map_or((secs, "s"), |(d, u)| (secs / d, u));
    format!("{n}{u}")
}

/// `words` checked and written the plain way.
pub fn normal(w: &str) -> Result<String, String> {
    secs(w).map(words)
}

/// The time it is due for: its last run (or, one that never ran, now) and its interval, once that has come. One run however many intervals passed: a daemon that was down does not run it again and again.
pub(super) fn owed(r: &Value, now: DateTime<Local>) -> Option<DateTime<Local>> {
    let every = TimeDelta::seconds(secs(&s(r, "every")).ok()?);
    let last = DateTime::parse_from_rfc3339(&s(r, "last")).ok();
    let next = last.map_or(now, |l| l.with_timezone(&Local) + every);
    (next <= now).then_some(next)
}

/// The card's `every` laid over routine `r`: set (normalised, and with no days, time or catch_up), or taken off with an empty one. True when `r` runs every so often now.
pub(super) fn apply(r: &mut Value, b: &Value) -> Result<bool, String> {
    if let Some(words) = b["every"].as_str().map(str::trim) {
        r["every"] = json!(if words.is_empty() {
            String::new()
        } else {
            normal(words)?
        });
    }
    let every = !s(r, "every").is_empty();
    if every {
        r["catch_up"] = json!(null);
        (r["days"], r["at"]) = (json!([]), json!("")); // a routine is every so often or at a time
    }
    Ok(every)
}
