//! Which time of its schedule a turn role's pass is owed for.
use super::pass::{mine, BORN};
use crate::app::Shared;
use chrono::{DateTime, Local};
use serde_json::json;

/// The time of the schedule whose pass `role` is to run for now: one, however many came.
pub fn owed_pass(app: &Shared, role: &str, now: DateTime<Local>) -> Option<DateTime<Local>> {
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
        app.kv(BORN).unwrap_or_default() // none yet: none is owed from before this NeoBot existed
    } else {
        last_daily
    };
    let passes = app.cfg().factory.passes().into_iter();
    passes
        .filter(|(to, ..)| to == role)
        .filter_map(|(_, at, days, catch_up)| {
            let route = json!({"at": at, "days": days, "catch_up": catch_up, "last": last_daily});
            crate::bots::owed(&route, now)
        })
        .max()
}
