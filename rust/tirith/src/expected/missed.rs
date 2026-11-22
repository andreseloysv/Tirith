//! Occurrences whose day is over and that nothing ever wrote down: Tirith was stopped, or the machine slept, across them. Each gets its row, and each routine one line.
use super::calendar::{local, on, over, utc};
use super::found::known;
use super::store::{close, expect, select, Expected, New, FAILED};
use super::timed::{timed, Timed};
use super::{gaps, lost, notice};
use crate::app::Shared;
use crate::server::s;
use chrono::{DateTime, Local};
use serde_json::{json, Value};

/// A day's end is looked at from this instant on for that routine (kv), with the schedule it had: a new or changed schedule, or one switched on, is watched from now, so nothing from before is ever invented.
fn watched(app: &Shared, t: &Timed, now: DateTime<Local>) -> (String, Option<DateTime<Local>>) {
    let key = format!("expected:watch:{}:{}:{}", t.conv, t.routine, t.at);
    let sig = json!([t.at, t.days, t.on]).to_string();
    let kept: Value = app
        .kv(&key)
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default();
    if s(&kept, "sig") == sig {
        return (key, local(&s(&kept, "since")).filter(|_| t.on));
    }
    app.set_kv(
        &key,
        Some(&json!({"sig": sig, "since": utc(now)}).to_string()),
    );
    (key, None)
}

pub(super) fn check(app: &Shared, now: DateTime<Local>) {
    for t in timed(app) {
        let (key, since) = watched(app, &t, now);
        let Some(since) = since else { continue };
        let missed: Vec<String> = over((&t.at, &t.days), since, now)
            .into_iter()
            .map(utc)
            .filter(|due| !known(app, &t.conv, &t.routine, due))
            .collect();
        write_down(app, &t, &missed);
        let midnight = on(now.date_naive(), "00:00").filter(|m| *m > since);
        if let Some(midnight) = midnight {
            let sig = json!([t.at, t.days, t.on]).to_string();
            app.set_kv(
                &key,
                Some(&json!({"sig": sig, "since": utc(midnight)}).to_string()),
            );
        }
    }
}

fn write_down(app: &Shared, t: &Timed, missed: &[String]) {
    let mut runs: Vec<Expected> = vec![];
    let mut whys: Vec<String> = vec![];
    for due in missed {
        let why = gaps::over(app, due).unwrap_or_else(|| "no llegó a empezar".into());
        let id = expect(
            app,
            &New {
                conv: &t.conv,
                routine: &t.routine,
                title: &t.title,
                trigger: "schedule",
                kind: "schedule",
                due,
                task: "",
            },
        );
        for run in select(app, "id = ?1", [id]) {
            let turn = lost::row(app, &run, &why);
            close(app, id, FAILED, turn, &why);
            runs.push(run);
        }
        whys.push(why);
    }
    if let Some(why) = whys.last() {
        notice::lost(app, &runs, why);
    }
}
