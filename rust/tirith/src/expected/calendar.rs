//! When something scheduled was due, in this machine's time, kept as UTC instants.
use chrono::{DateTime, Datelike, Days, Local, NaiveDate, NaiveTime, TimeDelta, TimeZone, Utc};

/// At most this many days back are looked through for what was missed.
const BACK: i64 = 31;

/// An instant as the store keeps it.
pub fn utc<T: TimeZone>(t: DateTime<T>) -> String {
    t.with_timezone(&Utc)
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

/// `kept`, in this machine's time.
pub(super) fn local(kept: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(kept)
        .ok()
        .map(|t| t.with_timezone(&Local))
}

/// `at` (HH:MM) of `day` here; in the hour a clock skips in spring, an hour later.
pub(super) fn on(day: NaiveDate, at: &str) -> Option<DateTime<Local>> {
    let t = day.and_time(NaiveTime::parse_from_str(at, "%H:%M").ok()?);
    // the first instant the clock shows it, as bots/schedule.rs names it
    let there = |t| {
        let shown = Local.from_local_datetime(&t);
        shown.earliest().min(shown.latest())
    };
    there(t).or_else(|| there(t + TimeDelta::hours(1)))
}

/// Today's occurrence at `at`, as kept.
pub fn today(at: &str, now: DateTime<Local>) -> Option<String> {
    on(now.date_naive(), at).map(utc)
}

fn is_day(day: NaiveDate, days: &[u64]) -> bool {
    days.contains(&u64::from(day.weekday().number_from_monday()))
}

/// The occurrences after `since` on the days that are over, the first first.
pub(super) fn over(
    (at, days): (&str, &[u64]),
    since: DateTime<Local>,
    now: DateTime<Local>,
) -> Vec<DateTime<Local>> {
    let today = now.date_naive();
    let mut day = since.date_naive().max(today - TimeDelta::days(BACK));
    let mut out = vec![];
    while day < today {
        let due = on(day, at).filter(|t| is_day(day, days) && *t > since);
        out.extend(due);
        day = day + Days::new(1);
    }
    out
}

/// Today's occurrence, when it came after `since` and is too late to run on time.
pub(super) fn gone_today(
    (at, days): (&str, &[u64]),
    since: DateTime<Local>,
    now: DateTime<Local>,
) -> Option<DateTime<Local>> {
    let today = now.date_naive();
    let time = on(today, at).filter(|t| is_day(today, days) && *t > since)?;
    crate::bots::past(time, now).then_some(time)
}

/// The first occurrence after `now`.
pub(super) fn next((at, days): (&str, &[u64]), now: DateTime<Local>) -> Option<DateTime<Local>> {
    (0..8u64)
        .map(|n| now.date_naive() + Days::new(n))
        .filter(|day| is_day(*day, days))
        .filter_map(|day| on(day, at))
        .find(|t| *t > now)
}
