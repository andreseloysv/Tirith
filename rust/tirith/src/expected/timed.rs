//! Everything that has a time of day: the bots' routines and the turn roles' scheduled passes.
use super::{PASS, PASS_TITLE};
use crate::app::Shared;
use crate::server::s;
use chrono::{DateTime, Local};
use serde_json::Value;

/// One of them.
pub(super) struct Timed {
    pub conv: String,
    pub routine: String,
    pub title: String,
    pub at: String,
    pub days: Vec<u64>,
    pub on: bool,
    /// `catch_up = "skip"`: late is lost.
    pub skip: bool,
    /// The time it is to run for now: that one is not lost.
    pub owed: Option<DateTime<Local>>,
}

/// All of them, switched off ones too.
pub(super) fn timed(app: &Shared, now: DateTime<Local>) -> Vec<Timed> {
    let mut out = vec![];
    for bot in crate::bots::all(app) {
        for r in bot["routines"].as_array().into_iter().flatten() {
            if s(r, "trigger") != "schedule" || s(r, "at").is_empty() {
                continue;
            }
            out.push(Timed {
                conv: s(&bot, "key"),
                routine: s(r, "id"),
                title: s(r, "title"),
                at: s(r, "at"),
                days: r["days"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_u64)
                    .collect(),
                on: r["on"] != false,
                skip: r["catch_up"] == "skip",
                owed: crate::bots::owed(r, now),
            });
        }
    }
    let cfg = app.cfg();
    let turn = |role: &str| cfg.factory.role::<String>(role, "session").as_deref() == Some("turn");
    for (role, at, days, catch_up) in cfg.factory.passes() {
        if turn(&role) {
            out.push(Timed {
                owed: crate::metrics::owed_pass(app, &role, now),
                skip: catch_up == "skip",
                conv: role,
                routine: PASS.into(),
                title: PASS_TITLE.into(),
                at,
                days,
                on: true,
            });
        }
    }
    out
}
