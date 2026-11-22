//! Today's timed routines of the bots: the ones done and the ones still to do.
use crate::app::Shared;
use crate::server::s;
use chrono::{DateTime, Datelike, Local, NaiveDateTime, NaiveTime};
use serde_json::{json, Value};
/// The counts, then one line per routine in the order of the day.
pub(super) fn routines(app: &Shared, now: DateTime<Local>) -> String {
    let mut rows: Vec<(String, String, &str)> = vec![];
    for bot in crate::bots::all(app) {
        for r in bot["routines"].as_array().into_iter().flatten() {
            let state = state(app, &s(&bot, "key"), r, now);
            let name = format!("{} · {}", s(&bot, "name"), s(r, "title"));
            rows.extend(state.map(|state| (s(r, "at"), name, state)));
        }
    }
    if rows.is_empty() {
        return "Routines today: none".into();
    }
    rows.sort();
    let count = |state: &str| rows.iter().filter(|r| r.2 == state).count();
    let (done, failed) = (count("done"), count("failed"));
    let mut head = format!("{done} done, {} pending", rows.len() - done - failed);
    if failed > 0 {
        head += &format!(", {failed} failed");
    }
    let lines: String = rows
        .iter()
        .map(|(at, name, state)| format!("\n- {at} {name}: {state}"))
        .collect();
    format!("Routines today: {head}{lines}")
}
/// Where routine `r` of bot `key` is today, or none when today is not its day. Its `last` stamp alone cannot say it ran: a change to a routine moves that stamp too.
fn state(app: &Shared, key: &str, r: &Value, now: DateTime<Local>) -> Option<&'static str> {
    let at = NaiveTime::parse_from_str(&s(r, "at"), "%H:%M").ok()?;
    let today = json!(now.weekday().number_from_monday());
    (r["trigger"] == "schedule" && r["days"].as_array()?.contains(&today)).then_some(())?;
    let slot = now.date_naive().and_time(at);
    if let Some(due) = crate::expected::today(&s(r, "at"), now) {
        if let Some(state) = crate::expected::stands(app, (key, &s(r, "id"), &due)) {
            return Some(state); // the register's word, as the numbers and the notice have it
        }
    }
    if let Some(fired) = fired(app, key, &s(r, "title"), slot) {
        return Some(outcome(app, key, &s(r, "id"), &fired));
    }
    let last = DateTime::parse_from_rfc3339(&s(r, "last"))
        .ok()
        .map(|t| t.with_timezone(&Local).naive_local());
    (r["on"] != false && last.is_none_or(|l| l < slot)).then_some("pending")
}
/// When the routine was handed to its bot for today's `slot`: its `schedule` event names it.
fn fired(app: &Shared, key: &str, title: &str, slot: NaiveDateTime) -> Option<String> {
    let sql = "SELECT at FROM events WHERE project = ?1 AND kind = 'schedule' AND instr(summary, ?2) > 0 AND datetime(at, 'localtime') >= ?3 ORDER BY id DESC LIMIT 1";
    let args = [key.into(), format!("«{title}»"), slot.to_string()];
    app.query(sql, args, |r| r.get(0)).into_iter().next()
}
/// How the routine's first run since `fired` ended: each run is a turn of its own, with the routine's id.
fn outcome(app: &Shared, key: &str, routine: &str, fired: &str) -> &'static str {
    let sql = "SELECT status FROM turns WHERE conv = ?1 AND role = ?1 AND routine = ?2 AND started_at >= ?3 ORDER BY id LIMIT 1";
    let ended: Vec<String> = app.query(sql, [key, routine, fired], |r| r.get(0));
    match ended.first().map(String::as_str) {
        Some("ok") => "done",
        Some("failed") => "failed",
        Some("waiting") => "waiting for you",
        _ => "started",
    }
}
