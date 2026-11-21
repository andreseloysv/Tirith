//! The chat's HTTP routes: clearing a conversation and its longest life.
use super::{clear::clear, keep::keep_of, keep::set_keep};
use crate::app::{bad, Shared};
use crate::server::s;
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::Value;

pub fn routes() -> Router<Shared> {
    Router::new()
        .route(
            "/api/chat/clear",
            post(
                |State(app): State<Shared>, Json(b): Json<Value>| async move {
                    clear(&app, &s(&b, "project")).map(Json).map_err(bad)
                },
            ),
        )
        .route(
            "/api/chat/keep",
            get(
                |State(app): State<Shared>,
                 Query(q): Query<std::collections::HashMap<String, String>>| async move {
                    Json(keep_of(&app, q.get("project").map_or("", String::as_str)))
                },
            )
            .post(
                |State(app): State<Shared>, Json(b): Json<Value>| async move {
                    set_keep(&app, &s(&b, "project"), &s(&b, "keep"))
                        .map(Json)
                        .map_err(bad)
                },
            ),
        )
}
