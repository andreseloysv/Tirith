//! Runs that wait, or work, too long (`[limits] routine_minutes`), counted on a clock that stops while the machine sleeps; and a pass whose day ended while it waited and that `catch_up` runs no more.
use super::calendar::local;
use super::life::never;
use super::store::{because, select, Expected, RUNNING, WAITING};
use super::PASS;
use crate::app::Shared;
use chrono::{DateTime, Local};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

type Key = (PathBuf, i64, String);
/// When each open run was first seen so, here.
static SEEN: Mutex<BTreeMap<Key, Instant>> = Mutex::new(BTreeMap::new());

fn minutes(app: &Shared) -> u64 {
    Some(app.cfg().factory.limits.routine_minutes)
        .filter(|m| *m > 0)
        .unwrap_or(60)
}

pub(super) fn check(app: &Shared, now: DateTime<Local>) {
    let open = select(app, "state IN (?1, ?2) ORDER BY id", [WAITING, RUNNING]);
    let most = minutes(app);
    let long: Vec<bool> = {
        let mut seen = SEEN.lock().unwrap();
        let key = |r: &Expected| (app.home.clone(), r.id, r.state.clone());
        let keys: Vec<Key> = open.iter().map(key).collect();
        seen.retain(|k, _| k.0 != app.home || keys.contains(k));
        keys.into_iter()
            .map(|k| {
                seen.entry(k)
                    .or_insert_with(Instant::now)
                    .elapsed()
                    .as_secs()
                    >= most * 60
            })
            .collect()
    };
    for (run, long) in open.iter().zip(long) {
        if run.routine == PASS {
            pass(app, run, now); // its turn has a limit of its own
        } else if long {
            stop(app, run, most);
        }
    }
}

/// A pass that still waits when its day is over never ran, unless it is the one still owed (caught up).
fn pass(app: &Shared, run: &Expected, now: DateTime<Local>) {
    let over = local(&run.due).is_some_and(|d| d.date_naive() < now.date_naive());
    let owed = || {
        let due = crate::metrics::owed_pass(app, &run.conv, now).map(super::calendar::utc);
        due.as_deref() == Some(run.due.as_str())
    };
    if run.state == WAITING && over && !owed() {
        let why = Some(run.why.as_str()).filter(|w| !w.is_empty());
        never(
            app,
            run,
            why.unwrap_or("no le llegó el turno en todo el día"),
        );
    }
}

fn stop(app: &Shared, run: &Expected, most: u64) {
    if run.state == WAITING {
        crate::manager::drop_run(&run.conv, &run.routine);
        let why = format!("no pudo empezar en {most} min: su conversación seguía ocupada o su agente no arrancaba");
        return never(app, run, &why);
    }
    let why = format!("tardó más de {most} min y NeoBot lo paró");
    because(app, run.id, &why); // its turn, ending, is judged by this
    if !crate::manager::stop_run(&run.conv) {
        crate::manager::drop_run(&run.conv, &run.routine); // not even its process: it never starts now
        never(app, run, &why);
    }
}
