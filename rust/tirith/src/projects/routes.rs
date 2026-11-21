//! The projects' HTTP routes and their list.
use super::{add::add, folders::folders, pick::pick};
use crate::app::{bad, Shared};
use axum::routing::{get, post};
use axum::{extract::State, Json, Router};
use serde_json::{json, Value};

pub fn list(app: &Shared) -> Vec<Value> {
    app.cfg().projects.iter().map(|p| json!({"id": p.id, "name": p.name, "color": p.color, "autopilot": crate::lanes::autopilot_on(app, &p.id), "main_red": app.kv(&format!("main_red:{}", p.id))})).collect()
}

pub fn routes() -> Router<Shared> {
    let add_route = |State(app): State<Shared>, Json(b): Json<Value>| async move {
        let s = |k: &str| crate::server::s(&b, k);
        add(
            &app,
            &s("folder"),
            &s("name"),
            &s("color"),
            b["dry"] == true,
        )
        .await
        .map(Json)
        .map_err(bad)
    };
    Router::new()
        .route("/api/projects", post(add_route))
        .route("/api/projects/folders", get(folders))
        .route("/api/projects/pick", post(pick))
}
