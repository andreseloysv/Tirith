//! The whole state the page starts from.
use crate::app::Shared;
use crate::{manager, projects};
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

pub(super) async fn state(State(app): State<Shared>) -> Json<Value> {
    Json(json!({
        "projects": projects::list(&app),
        "lanes": lanes(&app),
        "events": app.recent("events", &["id", "at", "lane", "project", "kind", "tool", "summary", "factory", "role_hash"], None, 200),
        "owner": manager::owner_open(&app),
        "busy": manager::busy(),
        "pid": std::process::id(),
        "factory_error": app.cfg().error,
    }))
}

/// Every lane, with what its runs spent (`spent`: runs, tokens, dollars) once it has run.
fn lanes(app: &Shared) -> Vec<Value> {
    let spent = crate::costs::by_lane(app);
    let lanes = app.lanes.lock().unwrap().clone();
    lanes
        .values()
        .map(|l| {
            let mut v = json!(l);
            v["spent"] = spent.get(&l.id).cloned().unwrap_or(Value::Null);
            v
        })
        .collect()
}
