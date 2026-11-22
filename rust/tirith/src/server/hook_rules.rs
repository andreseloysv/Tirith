//! A tool call that breaks a rule is refused before it runs.
use super::hook_lane::Call;
use crate::app::Shared;
use crate::{rules, sandbox};
use axum::Json;
use serde_json::{json, Value};

/// The deny answer for a call the rules (or, outside a lane and a role, the sandbox) refuse; none when it may run.
pub(super) fn refuse(app: &Shared, call: &Call) -> Option<Json<Value>> {
    let role = call.role.as_deref();
    let why = rules::check(
        app,
        &call.known,
        &call.input,
        call.lane.as_ref(),
        call.project.as_ref(),
        role,
    )
    .or_else(|| {
        if call.lane.is_none() && role.is_none() {
            sandbox::check_command(&call.known, &call.input)
        } else {
            None
        }
    })?;
    app.event(
        &call.lane_id,
        &call.project_id,
        "blocked",
        &call.tool,
        &format!("{} — {why}", call.summary),
        &call.session,
    );
    Some(Json(
        json!({"hookSpecificOutput": {"hookEventName": "PreToolUse",
            "permissionDecision": "deny", "permissionDecisionReason": why}}),
    ))
}
