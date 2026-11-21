//! The pipelines and their runs: the list, a retry, a stage's time limit and a run's log.
use super::super::settings::stage_limit;
use crate::app::{bad, tail, Shared};
use crate::merge;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;

pub fn runs() -> Router<Shared> {
    Router::new()
        .route(
            "/api/pipelines",
            get(
                |State(app): State<Shared>,
                 Query(q): Query<std::collections::HashMap<String, String>>| async move {
                    Json(json!(merge::list(
                        &app,
                        q.get("project").map_or("", String::as_str),
                        q.get("limit")
                            .and_then(|l| l.parse().ok())
                            .unwrap_or(30u32)
                            .min(200)
                    )))
                },
            ),
        )
        .route(
            "/api/pipelines/{id}/retry",
            post(
                |State(app): State<Shared>, Path(id): Path<String>| async move {
                    merge::retry(&app, &id)
                        .map(|_| Json(json!({"ok": true})))
                        .map_err(bad)
                },
            ),
        )
        .route("/api/stage-limit/{project}/{stage}", post(stage_limit))
        .route(
            "/api/runs/{id}/log",
            get(
                |State(app): State<Shared>, Path(id): Path<i64>| async move {
                    app.db
                        .lock()
                        .unwrap()
                        .query_row("SELECT log FROM runs WHERE id = ?1", [id], |r| {
                            r.get::<_, Option<String>>(0)
                        })
                        .ok()
                        .flatten()
                        .map(|l| tail(std::path::Path::new(&l), 400))
                        .unwrap_or_default()
                },
            ),
        )
}
