//! When the CEO pass may run by itself: its daily budget, and its scheduled daily pass.
use super::metric::iso;
use crate::app::Shared;
use serde_json::json;
/// Whether the CEO may still run by itself today: fewer reviews in 24 h (the owner's own not counted) than its `max_per_day`.
pub(super) fn under_budget(app: &Shared, ceo: &str, now: u64) -> bool {
    let max = app
        .cfg()
        .factory
        .role::<u64>(ceo, "max_per_day")
        .unwrap_or(6);
    let today = app
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM ceo_reviews WHERE at >= ?1 AND metric != 'owner'",
            [iso(now.saturating_sub(86400) as i64)],
            |r| r.get::<_, u64>(0),
        )
        .unwrap_or(0);
    today < max
}
/// The daily pass is configuration, not a decision here: the factory's `schedule` route to this role, due like a bot's routine when its time came today and no daily pass (kept as rows naming it) ran since.
pub(super) fn daily_due(app: &Shared, ceo: &str) -> bool {
    let last_daily = app
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT COALESCE(MAX(at), '') FROM ceo_reviews WHERE metric LIKE '%daily%'",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_default();
    let schedules = app.cfg().factory.schedules();
    schedules.into_iter().any(|(to, at, days)| {
        to == ceo
            && crate::bots::due(
                &json!({"at": at, "days": days, "last": last_daily}),
                chrono::Local::now(),
            )
    })
}
