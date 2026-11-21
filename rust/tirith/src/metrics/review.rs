//! The CEO pass: who reviews a long red stretch and when, the review turn itself, and the page's "CEO now".
use super::book::REVIEWING;
use super::brief::brief;
use crate::app::{now_iso, Shared};
use crate::manager;
use axum::{extract::State, Json};
use serde_json::json;
use std::sync::atomic::Ordering;
/// Who reviews a long red stretch, and after how many seconds: the turn role of the delayed `metric.red` route (its `after_minutes`, or the role's `review_after_minutes`); without one, any turn role when asked, and never by itself.
pub(super) fn delayed_review(app: &Shared) -> (String, u64) {
    let f = &app.cfg().factory;
    let turn =
        |r: &crate::factory::Route| f.role::<String>(&r.0, "session").as_deref() == Some("turn");
    f.routes_on("metric.red")
        .into_iter()
        .find(turn)
        .map_or_else(
            || (f.find_role("turn", None).unwrap_or_default(), u64::MAX),
            |r| {
                (
                    r.0.clone(),
                    r.4.or_else(|| f.role::<u64>(&r.0, "review_after_minutes"))
                        .unwrap_or(15)
                        * 60,
                )
            },
        )
}
/// The review's answer is kept with the stamp of its turn, and appears in the CEO's own conversation, as its answer there.
fn keep(app: &Shared, role: &str, metric: &str, result: &Result<String, String>) -> String {
    let text = result
        .clone()
        .unwrap_or_else(|e| format!("(the review could not run: {e})"));
    let (factory, role_hash) = app.stamp_for(role); // the review turn's stamp
    let _ = app.db.lock().unwrap().execute("INSERT INTO ceo_reviews (at, metric, text, factory, role_hash) VALUES (?1, ?2, ?3, ?4, ?5)", rusqlite::params![now_iso(), metric, text, factory, role_hash]);
    app.chat("manager", &text, role);
    text
}
pub(super) fn start_review(app: &Shared, role: String, metric: String, why: String) {
    let Some(()) = (!REVIEWING.swap(true, Ordering::SeqCst)).then_some(()) else {
        return;
    };
    let app = app.clone();
    tokio::spawn(async move {
        app.emit("review", json!({"running": true}));
        let result = manager::run_turn(&app, &role, &role, &brief(&app, &why)).await;
        let text = keep(&app, &role, &metric, &result);
        let _ = result.is_ok().then(|| {
            manager::emit_event(&app, &role, "review.done", &format!("Review: {text}"), None)
        });
        REVIEWING.store(false, Ordering::SeqCst);
        app.emit("review", json!({"running": false})); // the page refetches the metrics on it, reviews included
    });
}
/// The page's "CEO now": the CEO turn role runs at once, asked by the owner (it does not count toward its `max_per_day`).
pub(super) async fn review_now(State(app): State<Shared>) -> crate::app::R {
    let role = delayed_review(&app).0;
    (!role.is_empty())
        .then_some(())
        .ok_or_else(|| crate::app::bad("the factory has no turn role to review with".into()))?;
    start_review(
        &app,
        role,
        "owner".into(),
        "The owner asked for a CEO pass now.".into(),
    );
    Ok(Json(json!({"ok": true})))
}
