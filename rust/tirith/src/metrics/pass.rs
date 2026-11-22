//! When a turn role's pass may run by itself: its daily budget, and its scheduled pass.
use super::metric::iso;
use crate::app::Shared;
/// The passes of `role` in `ceo_reviews`: the rows naming it, and for the CEO (the role of the delayed `metric.red` route) the rows kept before passes named their role.
pub(super) fn mine(app: &Shared, role: &str) -> String {
    let ceo = super::review::delayed_review(app).0;
    format!(
        "(role = '{}'{})",
        role.replace('\'', ""),
        if role == ceo { " OR role = ''" } else { "" }
    )
}
/// Whether `role` may still run by itself today: fewer of its passes in 24 h (the owner's own not counted) than its `max_per_day`.
pub(super) fn under_budget(app: &Shared, role: &str, now: u64) -> bool {
    let max = app
        .cfg()
        .factory
        .role::<u64>(role, "max_per_day")
        .unwrap_or(6);
    let sql = format!(
        "SELECT COUNT(*) FROM ceo_reviews WHERE at >= ?1 AND metric != 'owner' AND {}",
        mine(app, role)
    );
    let today = app
        .db
        .lock()
        .unwrap()
        .query_row(&sql, [iso(now.saturating_sub(86400) as i64)], |r| {
            r.get::<_, u64>(0)
        })
        .unwrap_or(0);
    today < max
}
/// The scheduled pass is configuration, not a decision here: the factory's `schedule` route to this role, due like a bot's routine (its `catch_up` too), its last run being this role's last scheduled pass (kept as rows naming `daily`).
pub(super) fn daily_due(app: &Shared, role: &str) -> bool {
    super::owed::owed_pass(app, role, chrono::Local::now()).is_some()
}

/// Whether the scheduled pass of `role` may start now: due (and so expected, `expected/`) and within its budget; else it waits, saying for what.
pub(super) fn may_start(app: &Shared, role: &str, now: u64) -> bool {
    let due = daily_due(app, role);
    if !crate::expected::pass_due(app, role, due, chrono::Local::now()) {
        return false;
    }
    let under = under_budget(app, role, now);
    if !under {
        crate::expected::blocked(
            app,
            role,
            "su tope de pasadas en 24 h (max_per_day) estaba gastado",
        );
    }
    under
}

/// When this home's daemon first started.
pub(super) const BORN: &str = "born";

/// The daemon starts: the first time, it keeps when (for a home from before this was kept, its oldest event).
pub fn born(app: &Shared) {
    if app.kv(BORN).is_some() {
        return;
    }
    let sql = "SELECT COALESCE(MIN(at), '') FROM events";
    let oldest = app.query(sql, [], |r| r.get::<_, String>(0)).pop();
    let at = oldest
        .filter(|at| !at.is_empty())
        .unwrap_or_else(crate::app::now_iso);
    app.set_kv(BORN, Some(&at));
}

/// Whether this home is younger than `day` (the start of the last 24 h): a day's count has nothing to be held to yet.
pub(super) fn younger(app: &Shared, day: i64) -> bool {
    app.kv(BORN)
        .and_then(|at| super::metric::secs(&at))
        .is_some_and(|born| born > day)
}
