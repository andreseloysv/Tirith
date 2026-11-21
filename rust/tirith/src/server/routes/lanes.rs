//! A lane's routes: start, stop, delete, its log, its question and its answer.
use super::super::s;
use crate::app::{bad, tail, Shared};
use crate::lanes;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn lanes() -> Router<Shared> {
    Router::new()
        .route("/api/lanes", post(|State(app): State<Shared>, Json(b): Json<Value>| async move { lanes::start(&app, &s(&b, "project"), &s(&b, "step"), &s(&b, "item"), b["agent"].as_str().filter(|a| !a.is_empty()).map(String::from)).map(|l| Json(json!(l))).map_err(bad) }))
        .route("/api/lanes/{id}/stop", post(|State(app): State<Shared>, Path(id): Path<String>| async move { lanes::stop(&app, &id).map(|l| Json(json!(l))).map_err(bad) }))
        .route("/api/lanes/{id}/delete", post(|State(app): State<Shared>, Path(id): Path<String>| async move { lanes::delete(&app, &id).await.map(|_| Json(json!({"ok": true}))).map_err(bad) }))
        .route(
            "/api/lanes/{id}/log",
            get(|State(app): State<Shared>, Path(id): Path<String>| async move {
                crate::app::redact_secrets(&tail(&app.home.join(format!("logs/lanes/{}.log", id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect::<String>())), 400))
            }),
        )
        .route("/api/lanes/{id}/ask", post(|State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>| async move { lanes::ask(&app, &id, &s(&b, "question")).map(|l| Json(json!(l))).map_err(bad) }))
        .route("/api/lanes/{id}/answer", post(|State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>| async move { lanes::answer(&app, &id, &s(&b, "answer"), "the manager").map(|l| Json(json!(l))).map_err(bad) }))
}
