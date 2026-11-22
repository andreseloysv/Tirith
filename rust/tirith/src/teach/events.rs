//! The extension's steps: one, or `{"steps": [...]}`, kept only while recording, each masked again here and timed on the daemon's clock.
use super::api::key;
use super::clock::at;
use super::cors::allow;
use super::mask::kept;
use super::record::stop_now;
use super::state::{left_ms, live, now_ms, push, read};
use crate::app::Shared;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::Json;
use serde_json::{json, Value};
pub(super) async fn events(
    State(app): State<Shared>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    let key = match key(&app, &h, &id, true) {
        Ok(k) => k,
        Err(r) => return *r,
    };
    let rec = read(&app, &key);
    if rec["on"] == true && !live(&rec) {
        stop_now(&app, &key, ""); // its 10 minutes are over
    }
    let b: Value = serde_json::from_slice(&body).unwrap_or_default();
    let steps = b["steps"].as_array().cloned().unwrap_or_else(|| vec![b]);
    let now = now_ms();
    let took = steps
        .iter()
        .filter_map(|st| kept(st, &at(st, &now)))
        .filter(|st| push(&app, &key, st.clone()))
        .count();
    let _ = (took > 0).then(|| app.emit("teach", json!({"bot": key})));
    let rec = read(&app, &key);
    allow(
        &h,
        Json(json!({"ok": true, "kept": took, "on": live(&rec), "left_ms": left_ms(&rec)})),
    )
}
