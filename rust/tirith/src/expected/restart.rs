//! What a start finds open: a run Tirith was stopped in the middle of failed and is not done again (it may have acted); one that had not started is handed again on the day it was due or caught up, and else is lost.
use super::calendar::local;
use super::life::never;
use super::store::{close, select, Expected, FAILED, OK, RUNNING, WAITING};
use super::{notice, PASS};
use crate::app::Shared;
use chrono::{Local, TimeDelta};
use rusqlite::params;

/// How far back `catch_up` looks (bots/schedule.rs).
const CAUGHT_UP: i64 = 7;
const MIDDLE: &str = "Tirith se paró o se reinició a mitad";

/// At the daemon's start, after the rooms are open: the phones hear it.
pub fn start(app: Shared) {
    for run in select(&app, "state IN (?1, ?2) ORDER BY id", [WAITING, RUNNING]) {
        if run.state == RUNNING {
            cut(&app, &run);
        } else if run.routine != PASS {
            again(&app, &run); // a waiting pass is the next tick's
        }
    }
}

/// It was running: the row its turn left (`turns/interrupted.rs`), or one of its own.
fn cut(app: &Shared, run: &Expected) {
    let sql = "SELECT id, COALESCE(status, '') FROM turns WHERE conv = ?1 AND role = ?1 AND routine = ?2 AND started_at >= ?3 ORDER BY id DESC LIMIT 1";
    let left = app.query(sql, params![run.conv, run.routine, run.started_at], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    });
    let Some((turn, status)) = left.into_iter().next() else {
        return never(app, run, MIDDLE);
    };
    if status != "failed" {
        return close(app, run.id, OK, turn, "");
    }
    close(app, run.id, FAILED, turn, MIDDLE);
    app.insert(
        "UPDATE turns SET why = ?1 WHERE id = ?2",
        params![MIDDLE, turn],
    );
    notice::failed(app, run, MIDDLE);
}

/// It had not started.
fn again(app: &Shared, run: &Expected) {
    let now = Local::now().date_naive();
    let is_now = |kept: &str| local(kept).is_some_and(|d| d.date_naive() == now);
    let caught_up = is_now(&run.created_at)
        && local(&run.due).is_some_and(|d| now - d.date_naive() <= TimeDelta::days(CAUGHT_UP));
    let today = is_now(&run.due) || caught_up;
    let handed = today
        && crate::manager::run_routine(
            app,
            &run.conv,
            &run.kind,
            &run.task,
            (&run.trigger, &run.routine),
        );
    if !handed {
        never(app, run, "Tirith se paró antes de que empezara");
    }
}
