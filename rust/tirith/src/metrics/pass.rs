//! When a turn role's pass may run by itself: its daily budget, and its scheduled pass.
use super::metric::iso;
use crate::app::Shared;
use serde_json::json;
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
/// The scheduled pass is configuration, not a decision here: the factory's `schedule` route to this role, due like a bot's routine when its time came and no scheduled pass of this role (kept as rows naming `daily`) ran since.
pub(super) fn daily_due(app: &Shared, role: &str) -> bool {
    let sql = format!(
        "SELECT COALESCE(MAX(at), '') FROM ceo_reviews WHERE metric LIKE '%daily%' AND {}",
        mine(app, role)
    );
    let last_daily = app
        .db
        .lock()
        .unwrap()
        .query_row(&sql, [], |r| r.get::<_, String>(0))
        .unwrap_or_default();
    let last_daily = if last_daily.is_empty() {
        app.kv(BORN).unwrap_or_default() // none yet: none is owed from before this Tirith existed
    } else {
        last_daily
    };
    let schedules = app.cfg().factory.schedules();
    schedules.into_iter().any(|(to, at, days)| {
        to == role
            && crate::bots::due(
                &json!({"at": at, "days": days, "last": last_daily}),
                chrono::Local::now(),
            )
    })
}

/// When this home's daemon first started.
const BORN: &str = "born";

/// The daemon starts: the first time, it keeps when.
pub fn born(app: &Shared) {
    if app.kv(BORN).is_none() {
        app.set_kv(BORN, Some(&crate::app::now_iso()));
    }
}
