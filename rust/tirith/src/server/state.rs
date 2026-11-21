//! The whole state the page starts from.
use crate::app::Shared;
use crate::{manager, projects};
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

pub(super) async fn state(State(app): State<Shared>) -> Json<Value> {
    Json(json!({
        "projects": projects::list(&app),
        "lanes": app.lanes.lock().unwrap().values().map(|l| json!(l)).collect::<Vec<_>>(),
        "events": app.recent("events", &["id", "at", "lane", "project", "kind", "tool", "summary", "factory", "role_hash"], None, 200),
        "owner": manager::owner_open(&app),
        "busy": manager::busy(),
        "factory_error": app.cfg().error,
    }))
}
