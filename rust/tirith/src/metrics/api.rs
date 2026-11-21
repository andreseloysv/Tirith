//! The metrics' clock and the page's API: the computation every minute, the last result and the reviews.
use super::book::{BOOK, REVIEWING};
use super::review::review_now;
use super::tick::tick;
use crate::app::Shared;
use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::time::Duration;
pub(super) fn payload(app: &Shared) -> Value {
    json!({"metrics": BOOK.lock().unwrap().last.as_array().cloned().unwrap_or_default(), "reviews": app.recent("ceo_reviews", &["id", "at", "metric", "text", "factory", "role_hash"], None, 10), "reviewing": REVIEWING.load(Ordering::SeqCst)})
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
