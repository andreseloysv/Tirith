//! Occurrences that nothing ever wrote down and that will not run: NeoBot was stopped, or the machine slept, across them. The one `catch_up` still runs is not lost; with "skip" one is lost as soon as it is late. Each gets its row, and each routine one line.
use super::calendar::{gone_today, local, on, over, utc};
use super::found::known;
use super::store::{close, expect, select, Expected, New, FAILED};
use super::timed::{timed, Timed};
use super::{gaps, lost, notice};
use crate::app::Shared;
use crate::server::s;
use chrono::{DateTime, Local, TimeDelta};
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
    for t in timed(app, now) {
        let (key, since) = watched(app, &t, now);
        let Some(since) = since else { continue };
        let written = |due: DateTime<Local>| known(app, &t.conv, &t.routine, &utc(due));
        let to_run = t.owed.filter(|due| !written(*due));
        let mut gone = over((&t.at, &t.days), since, now);
        if t.skip {
            gone.extend(gone_today((&t.at, &t.days), since, now));
        }
        gone.retain(|due| Some(*due) != to_run && !written(*due));
        let missed: Vec<String> = gone.into_iter().map(utc).collect();
        write_down(app, &t, &missed);
        // never past the one that is still to run, until it is written down
        let looked = on(now.date_naive(), "00:00")
            .map(|day| to_run.map_or(day, |due| day.min(due - TimeDelta::minutes(1))));
        if let Some(looked) = looked.filter(|looked| *looked > since) {
            let sig = json!([t.at, t.days, t.on]).to_string();
            app.set_kv(
                &key,
                Some(&json!({"sig": sig, "since": utc(looked)}).to_string()),
            );
        }
    }
}

fn write_down(app: &Shared, t: &Timed, missed: &[String]) {
    let mut runs: Vec<Expected> = vec![];
    let mut whys: Vec<String> = vec![];
    for due in missed {
        let why = gaps::over(app, due).unwrap_or_else(|| {
            let skips = if t.skip {
                " a su hora, y su catch_up es skip"
            } else {
                ""
            };
            format!("no llegó a empezar{skips}")
        });
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
