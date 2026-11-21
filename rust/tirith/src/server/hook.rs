//! One agent hook call: record it, refuse a tool call that breaks a rule, hand a lane its live messages, and keep a lane from ending its turn on a question.
use super::{hook_lane::Call, hook_reply, hook_rules};
use crate::app::Shared;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde_json::{json, Value};

pub(super) async fn hook(
    State(app): State<Shared>,
    Path(event): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Json<Value> {
    let p: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let call = Call::read(&app, &event, &headers, &p);
    call.note_activity(&app);
    call.record_access(&app);
    if event == "PreToolUse" {
        if let Some(denied) = hook_rules::refuse(&app, &call, &headers) {
            return denied;
        }
    }
    let _ = (event != "PostToolUse").then(|| {
        app.event(
            &call.lane_id,
            &call.project_id,
            &event,
            &call.tool,
            &call.summary,
            &call.session,
        )
    });
    let Some(l) = call.lane else {
        return Json(json!({}));
    };
    hook_reply::reply(&app, &event, &l, &p)
}

/// A header an agent filled in (an unset `$VAR` it passed through as is counts as none).
pub(super) fn header(headers: &HeaderMap, k: &str) -> Option<String> {
    headers
        .get(k)
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty() && !v.starts_with('$'))
        .map(String::from)
}
