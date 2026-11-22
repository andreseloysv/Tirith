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
        // A replay's call that reads outside is answered from its capture, as the reason it is refused; a shadow run's outward action is refused.
        let lab = crate::lab::answer(&call.cwd, &call.tool, &call.known, &call.input);
        let lab =
            lab.or_else(|| crate::lab::refused(&call.cwd, &call.tool, &call.known, &call.input));
        if let Some(why) = lab {
            return Json(json!({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                "permissionDecision": "deny", "permissionDecisionReason": why}}));
        }
        if let Some(denied) = hook_rules::refuse(&app, &call, &headers) {
            return denied;
        }
    }
    // a dot is in the daemon's own kinds (`owner.message`): no caller writes one of those
    let _ = (event != "PostToolUse" && !event.contains('.')).then(|| {
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

/// The hook as a Codex under its caller's limit has it (`hook_door.rs`): the rules' answer before a tool call, with no lane's state, message or note, and no replay's captured answer.
#[cfg(any(test, target_os = "macos"))]
pub(super) async fn rules_alone(
    State(app): State<Shared>,
    Path(event): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Json<Value> {
    if event != "PreToolUse" {
        return Json(json!({}));
    }
    let p: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let call = Call::read(&app, &event, &headers, &p);
    if let Some(why) = crate::lab::refused(&call.cwd, &call.tool, &call.known, &call.input) {
        return Json(json!({"hookSpecificOutput": {"hookEventName": "PreToolUse",
            "permissionDecision": "deny", "permissionDecisionReason": why}}));
    }
    hook_rules::refuse(&app, &call, &headers).unwrap_or(Json(json!({})))
}

/// A header an agent filled in (an unset `$VAR` it passed through as is counts as none).
pub(super) fn header(headers: &HeaderMap, k: &str) -> Option<String> {
    headers
        .get(k)
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty() && !v.starts_with('$'))
        .map(String::from)
}
