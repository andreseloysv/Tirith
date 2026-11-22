//! The pass of a turn role (the CEO, the Coach): who reviews a long red stretch and when, the pass turn itself with the brief its role names, and the page's "CEO now".
use super::book::RUNNING;
use crate::app::{now_iso, Shared};
use crate::manager;
use axum::{extract::State, Json};
use serde_json::json;
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
/// The pass's answer is kept with the stamp of its turn and its role, and appears in the role's own conversation, as its answer there.
fn keep(app: &Shared, role: &str, metric: &str, result: &Result<String, String>) -> String {
    let text = result
        .clone()
        .unwrap_or_else(|e| format!("(the review could not run: {e})"));
    let (factory, role_hash) = app.stamp_for(role); // the review turn's stamp
    let _ = app.db.lock().unwrap().execute("INSERT INTO ceo_reviews (at, metric, text, factory, role_hash, role) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", rusqlite::params![now_iso(), metric, text, factory, role_hash, role]);
    if result.is_ok() || !scheduled(metric) {
        app.chat("manager", &text, role); // a scheduled one's failure is the one line of `expected/`
    }
    text
}
/// Whether the role's schedule is (one of) what started the pass.
fn scheduled(metric: &str) -> bool {
    metric.split(',').any(|m| m == "daily")
}
/// What started a pass, for its turn's row: its schedule, the event it heard, or (a red stretch, the owner's "CEO now") nobody said.
fn trigger(app: &Shared, role: &str, metric: &str) {
    let (by, routine) = match metric.strip_prefix("event:") {
        Some(on) => (format!("event:{on}"), ""),
        None if scheduled(metric) => {
            crate::expected::started(app, role, crate::expected::PASS);
            ("schedule".to_string(), crate::expected::PASS)
        }
        None => return,
    };
    crate::turns::trigger(&format!("turn:{role}"), &by, routine);
}
/// One pass of turn role `role` (one at a time per role), with the brief its `brief` key names and `why` it runs.
pub(super) fn start_review(app: &Shared, role: String, metric: String, why: String) {
    let Some(()) = RUNNING.lock().unwrap().insert(role.clone()).then_some(()) else {
        return super::queued::keep(&role, &metric, &why); // an event heard during a pass is run when it ends
    };
    let app = app.clone();
    tokio::spawn(async move {
        app.emit("review", json!({"running": true, "role": role}));
        trigger(&app, &role, &metric);
        let brief = super::roles::brief_for(&app, &role, &why);
        let result = manager::run_turn(&app, &role, &role, &brief).await;
        let text = keep(&app, &role, &metric, &result);
        // The factory's review (the "ceo" brief) is told to the manager of no project; another role's answer stays in its own conversation.
        let told = result.is_ok() && super::roles::brief_key(&app, &role) == "ceo";
        let _ = told.then(|| {
            manager::emit_event(&app, &role, "review.done", &format!("Review: {text}"), None)
        });
        RUNNING.lock().unwrap().remove(&role);
        app.emit("review", json!({"running": false, "role": role})); // the page refetches the metrics on it, reviews included
        if let Some((metric, why)) = super::queued::take(&role) {
            start_review(&app, role, metric, why);
        }
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
