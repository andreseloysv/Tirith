//! Settings the manager or the page change: a project's autopilot and a stage's time limit.
use crate::app::{bad, now_s, Shared, R};
use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

pub(super) async fn autopilot(
    State(app): State<Shared>,
    Path(project): Path<String>,
    Json(b): Json<Value>,
) -> R {
    app.cfg()
        .project(&project)
        .ok_or(bad(format!("unknown project '{project}'")))?;
    let on = b["on"].as_bool().unwrap_or(false);
    app.set_kv(
        &format!("autopilot:{project}"),
        Some(if on { "on" } else { "off" }),
    );
    app.emit("autopilot", json!({"project": project, "on": on}));
    app.sched_wake.notify_one();
    Ok(Json(json!({"project": project, "on": on})))
}

/// The manager sets a stage's time limit in minutes; it lasts until changed or 24 h.
pub(super) async fn stage_limit(
    State(app): State<Shared>,
    Path((project, stage)): Path<(String, String)>,
    Json(b): Json<Value>,
) -> R {
    let minutes = b["minutes"]
        .as_u64()
        .filter(|m| *m > 0)
        .ok_or_else(|| bad("minutes must be a positive number".into()))?;
    app.set_kv(
        &format!("stage_limit:{project}:{stage}"),
        Some(&format!("{minutes}@{}", now_s())),
    );
    Ok(Json(
        json!({"project": project, "stage": stage, "minutes": minutes}),
    ))
}
