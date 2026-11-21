//! The guard's HTTP API: every project's state, and installing one.
use super::{install, intact, words};
use crate::app::Shared;
use crate::config::Project;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{http::StatusCode, Json, Router};
use serde_json::{json, Value};

async fn list(State(app): State<Shared>) -> Json<Value> {
    let row = |p: &Project| {
        let (n, r) = (words(&app, p).len(), intact(&app, p));
        json!({"project": p.id, "name": p.name, "words": n, "ok": r.is_ok(), "reason": r.err().unwrap_or_else(|| if n == 0 { "sin guarda".into() } else { "ok".into() })})
    };
    Json(json!(app
        .cfg()
        .projects
        .iter()
        .map(row)
        .collect::<Vec<_>>()))
}

pub fn routes() -> Router<Shared> {
    let install_one = |State(app): State<Shared>, Path(project): Path<String>| async move {
        let p = app
            .cfg()
            .project(&project)
            .cloned()
            .ok_or((StatusCode::NOT_FOUND, format!("unknown project {project}")))?;
        install(&app, &p).map_err(|e| (StatusCode::CONFLICT, e))?;
        Ok::<_, (StatusCode, String)>(Json(json!({"project": project, "ok": true})))
    };
    Router::new()
        .route("/api/guard", get(list))
        .route("/api/guard/{project}/install", post(install_one))
}
