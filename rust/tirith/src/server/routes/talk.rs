//! Talking: a conversation's rows, a new line in it, interrupting its manager, and the agents' hooks.
use super::super::{chat::chat, hook::hook, s};
use crate::app::{bad, Shared};
use crate::manager;
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn talk() -> Router<Shared> {
    Router::new()
        // The last rows of one conversation: `?project=<id>`, or the one of no project.
        .route(
            "/api/chat",
            get(
                |State(app): State<Shared>,
                 Query(q): Query<std::collections::HashMap<String, String>>| async move {
                    Json(json!(app.recent(
                        "chat",
                        &[
                            "id",
                            "at",
                            "role",
                            "text",
                            "project",
                            "factory",
                            "role_hash"
                        ],
                        Some(app.conv(q.get("project").map_or("", String::as_str))),
                        300
                    )))
                },
            )
            .post(chat),
        )
        .route(
            "/api/manager/interrupt",
            post(
                |State(app): State<Shared>, Json(b): Json<Value>| async move {
                    manager::interrupt(app.conv(&s(&b, "project")))
                        .map(|_| Json(json!({"ok": true})))
                        .map_err(bad)
                },
            ),
        )
        .route("/hook/{event}", post(hook))
}
