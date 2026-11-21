//! The metrics' clock and the page's API: the computation every minute, the last result and the reviews.
use super::book::{BOOK, RUNNING};
use super::review::review_now;
use super::tick::tick;
use crate::app::Shared;
use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};
use std::time::Duration;
/// The page's metrics, and the CEO's last reviews (another turn role's passes are in its own conversation) and whether one runs.
pub(super) fn payload(app: &Shared) -> Value {
    let ceo = super::review::delayed_review(app).0;
    let cols = ["id", "at", "metric", "text", "factory", "role_hash"];
    let sql = format!("SELECT {c} FROM (SELECT {c} FROM ceo_reviews WHERE {} ORDER BY id DESC LIMIT 10) ORDER BY id", super::pass::mine(app, &ceo), c = cols.join(", "));
    json!({"metrics": BOOK.lock().unwrap().last.as_array().cloned().unwrap_or_default(), "reviews": app.objects(&sql, [], &cols), "reviewing": RUNNING.lock().unwrap().contains(&ceo)})
}
pub fn start(app: Shared) {
    tokio::spawn(async move {
        loop {
            tick(&app);
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}
pub fn routes() -> Router<Shared> {
    Router::new()
        .route(
            "/api/metrics",
            get(|State(app): State<Shared>| async move { Json(payload(&app)) }),
        )
        .route("/api/review", axum::routing::post(review_now))
}
