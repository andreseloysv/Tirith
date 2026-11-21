//! The owner's cards: asked, answered, closed.
use super::super::s;
use crate::app::{bad, Shared};
use crate::manager;
use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::Value;

pub fn owner() -> Router<Shared> {
    Router::new()
        .route("/api/owner", post(|State(app): State<Shared>, Json(b): Json<Value>| async move { manager::ask_owner(&app, &s(&b, "project"), &s(&b, "kind"), &s(&b, "question"), &s(&b, "lane")).map(Json).map_err(bad) }))
        .route("/api/owner/{id}", post(|State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>| async move { manager::owner_answer(&app, &id, &s(&b, "answer"), s(&b, "by") == "manager").map(Json).map_err(bad) }))
        .route("/api/owner/{id}/close", post(|State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>| async move { manager::close_card(&app, &id, if s(&b, "by") == "manager" { "the manager" } else { "the owner" }, &s(&b, "why")).map(Json).map_err(bad) }))
}
