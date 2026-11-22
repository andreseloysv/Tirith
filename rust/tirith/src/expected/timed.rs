//! Everything that has a time of day: the bots' routines and the turn roles' scheduled passes.
use super::{PASS, PASS_TITLE};
use crate::app::Shared;
use crate::server::s;
use serde_json::Value;

/// One of them.
pub(super) struct Timed {
    pub conv: String,
    pub routine: String,
    pub title: String,
    pub at: String,
    pub days: Vec<u64>,
    pub on: bool,
}

/// All of them, switched off ones too.
pub(super) fn timed(app: &Shared) -> Vec<Timed> {
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
            });
        }
    }
    let cfg = app.cfg();
    let turn = |role: &str| cfg.factory.role::<String>(role, "session").as_deref() == Some("turn");
    for (role, at, days) in cfg.factory.schedules() {
        if turn(&role) {
            out.push(Timed {
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
