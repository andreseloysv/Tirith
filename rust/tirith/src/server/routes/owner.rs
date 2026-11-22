//! The owner's cards: asked, answered, closed.
use super::super::s;
use crate::app::{bad, Shared, R};
use crate::manager;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::Value;

pub fn owner() -> Router<Shared> {
    Router::new()
        .route(
            "/api/owner",
            post(
                |State(app): State<Shared>, Json(b): Json<Value>| async move {
                    manager::ask_owner(
                        &app,
                        &s(&b, "project"),
                        &s(&b, "kind"),
                        &s(&b, "question"),
                        &s(&b, "lane"),
                    )
                    .map(Json)
                    .map_err(bad)
                },
            ),
        )
        .route("/api/owner/{id}", post(answer))
        .route("/api/owner/{id}/close", post(close))
}

/// The owner's answer; an agent's is only what it may hand in for them (`manager/relayed.rs`).
async fn answer(
    State(app): State<Shared>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(b): Json<Value>,
) -> R {
    let (text, by_manager) = (s(&b, "answer"), s(&b, "by") == "manager");
    let done = if super::super::agent(&h) {
        manager::relayed(&app, &id, &text)
    } else {
        manager::owner_answer(&app, &id, &text, by_manager)
    };
    done.map(Json).map_err(bad)
}

async fn close(
    State(app): State<Shared>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(b): Json<Value>,
) -> R {
    let manager = super::super::agent(&h) || s(&b, "by") == "manager";
    let by = if manager { "the manager" } else { "the owner" };
    manager::close_card(&app, &id, by, &s(&b, "why"))
        .map(Json)
        .map_err(bad)
}
