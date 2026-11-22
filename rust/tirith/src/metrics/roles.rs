//! Any turn role's pass, not only the CEO's: the brief its `brief` key names ("ceo" or "bots"), its `schedule` route and the events routed to it, each within its `max_per_day`.
use super::pass::{may_start, under_budget};
use super::review::start_review;
use crate::app::{now_s, Shared};

/// The brief a turn role reads: its `brief` key, "ceo" when it names none.
pub(super) fn brief_key(app: &Shared, role: &str) -> String {
    app.cfg()
        .factory
        .role::<String>(role, "brief")
        .unwrap_or_else(|| "ceo".into())
}

/// The brief of a pass of `role`, run because of `why`.
pub(super) fn brief_for(app: &Shared, role: &str, why: &str) -> String {
    match brief_key(app, role).as_str() {
        "bots" => super::bots_brief::brief(app, role, why),
        _ => super::brief::brief(app, why),
    }
}

/// A role that runs as one fresh turn with a prompt of its own (the judge has none: each rubric is its instructions).
fn is_turn(app: &Shared, role: &str) -> bool {
    let f = &app.cfg().factory;
    f.role::<String>(role, "session").as_deref() == Some("turn") && !f.prompts_of(role).is_empty()
}

/// Every turn role but `ceo` (whose scheduled pass runs with the red measures) whose `schedule` route is due runs its scheduled pass, within its budget.
pub(super) fn scheduled(app: &Shared, ceo: &str, now: u64) {
    let mut roles: Vec<String> = app
        .cfg()
        .factory
        .schedules()
        .into_iter()
        .map(|s| s.0)
        .collect();
    roles.sort();
    roles.dedup();
    roles
        .into_iter()
        .filter(|r| r != ceo && is_turn(app, r) && may_start(app, r, now))
        .for_each(|r| start_review(app, r, "daily".into(), "Scheduled pass.".into()));
}

/// An event routed as a turn to a turn role, by a route with no `after_minutes` (the delayed `metric.red` one is the red measures' pass, run by the tick): its pass runs now, within its budget.
pub fn heard(app: &Shared, role: &str, on: &str, text: &str) {
    let ok = on != "schedule" && is_turn(app, role) && under_budget(app, role, now_s());
    let _ = ok.then(|| {
        start_review(
            app,
            role.into(),
            format!("event:{on}"),
            format!("{on}: {text}"),
        )
    });
}
