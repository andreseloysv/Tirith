//! What the page reads: the whole state, the change stream, a project's plan and its autopilot.
use super::super::{
    plan::{plan_show, plan_step, plan_step_delete},
    s,
    settings::autopilot,
    state::state as whole,
};
use crate::app::{bad, Shared};
use crate::plan;
use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::convert::Infallible;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

pub fn state() -> Router<Shared> {
    Router::new()
        .route("/api/state", get(whole))
        .route("/api/stream", get(|State(app): State<Shared>| async move { Sse::new(BroadcastStream::new(app.tx.subscribe()).filter_map(|m| m.ok().map(|d| Ok::<Event, Infallible>(Event::default().data(d))))).keep_alive(KeepAlive::default()) }))
        .route(
            "/api/plan/{project}",
            get(plan_show).post(|State(app): State<Shared>, Path(project): Path<String>, Json(b): Json<Value>| async move { plan::add(&app, &project, &s(&b, "step"), &s(&b, "text")).await.map(|_| Json(json!({"ok": true}))).map_err(bad) }),
        )
        .route("/api/plan/{project}/steps", post(plan_step))
        .route("/api/plan/{project}/steps/{num}/delete", post(plan_step_delete))
        .route("/api/autopilot/{project}", post(autopilot))
}
