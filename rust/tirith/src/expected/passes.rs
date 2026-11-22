//! A turn role's scheduled pass as an expected run: written down when due, waiting while something keeps it, and never twice.
use super::found::{occurrence, oldest};
use super::store::{because, expect, New, WAITING};
use super::{calendar, PASS, PASS_TITLE};
use crate::app::Shared;
use chrono::{DateTime, Local};

/// Whether the pass of `role` due today is still to be started (`due`: no pass of it ran since its time); it is written down the first time.
pub fn pass_due(app: &Shared, role: &str, due: bool, now: DateTime<Local>) -> bool {
    if !due {
        return false;
    }
    let times: Vec<(String, Vec<u64>)> = app
        .cfg()
        .factory
        .passes()
        .into_iter()
        .filter(|(to, ..)| to == role)
        .map(|(_, at, days, _)| (at, days))
        .collect();
    let Some(at) = calendar::latest(&times, now) else {
        return false;
    };
    if let Some(run) = occurrence(app, (role, PASS, &at)) {
        return run.state == WAITING; // one that started, or failed, is not started again
    }
    expect(
        app,
        &New {
            conv: role,
            routine: PASS,
            title: PASS_TITLE,
            trigger: "schedule",
            kind: "schedule",
            due: &at,
            task: "",
        },
    );
    true
}

/// The due pass of `role` waits for `why`.
pub fn blocked(app: &Shared, role: &str, why: &str) {
    if let Some(run) = oldest(app, role, PASS, WAITING) {
        because(app, run.id, why);
    }
}
