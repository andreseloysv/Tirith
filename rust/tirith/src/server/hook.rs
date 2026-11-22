//! One agent hook call: record it, refuse a tool call that breaks a rule, hand a lane its live messages, and keep a lane from ending its turn on a question. A call without an agent's token (the owner's own sessions) can only be refused.
use super::{hook_lane::Call, hook_reply, hook_rules, who::Agent, Who};
use crate::app::Shared;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

pub(super) async fn hook(
    State(app): State<Shared>,
    Path(event): Path<String>,
    who: Who,
    body: Bytes,
) -> Json<Value> {
    let p: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let Some(agent) = who.agent() else {
        return refusals(&app, &event, None, &p);
    };
    let call = Call::read(&app, &event, Some(agent), &p);
    call.note_activity(&app);
    call.record_access(&app);
    if event == "PreToolUse" {
        // A replay's call that reads outside is answered from its capture, as the reason it is refused; a shadow run's outward action is refused.
        let lab = crate::lab::answer(&call.cwd, &call.tool, &call.known, &call.input);
        let lab =
            lab.or_else(|| crate::lab::refused(&call.cwd, &call.tool, &call.known, &call.input));
        if let Some(why) = lab {
            return denied(&why);
        }
        if let Some(denied) = hook_rules::refuse(&app, &call) {
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
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Json<Value> {
    let p: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let agent = super::identify::agent(&app, &headers);
    refusals(&app, &event, agent.as_ref(), &p)
}

/// The rules' and the block list's refusals before a tool call, and nothing else of the hook.
fn refusals(app: &Shared, event: &str, agent: Option<&Agent>, p: &Value) -> Json<Value> {
    if event != "PreToolUse" {
        return Json(json!({}));
    }
    let call = Call::read(app, event, agent, p);
    if let Some(why) = crate::lab::refused(&call.cwd, &call.tool, &call.known, &call.input) {
        return denied(&why);
    }
    hook_rules::refuse(app, &call).unwrap_or(Json(json!({})))
}

fn denied(why: &str) -> Json<Value> {
    Json(json!({"hookSpecificOutput": {"hookEventName": "PreToolUse",
        "permissionDecision": "deny", "permissionDecisionReason": why}}))
}
