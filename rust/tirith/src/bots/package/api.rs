//! The package routes, all the owner's alone: export a bot, preview a package, import one.
use crate::app::{bad, Shared, R};
use crate::server::s;
use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::Value;

async fn blocking(work: impl FnOnce() -> Result<Value, String> + Send + 'static) -> R {
    let out = tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| bad("the work stopped".into()))?;
    out.map(Json).map_err(bad)
}

pub(in crate::bots) fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/bots/{id}/export", post(|State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>| async move {
            blocking(move || super::export::export(&app, &id, &b)).await
        }))
        .route("/api/bots/package/preview", post(|Json(b): Json<Value>| async move { blocking(move || super::preview::preview(&b["package"])).await }))
        .route("/api/bots/package/import", post(|State(app): State<Shared>, Json(b): Json<Value>| async move {
            blocking(move || super::import::import(&app, &b["package"], &s(&b, "hash"))).await
        }))
        .route_layer(axum::middleware::from_fn(crate::bots::owner::owner_only))
}
