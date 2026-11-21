//! What a lane's agent hears back from a hook call that was not refused.
use super::s;
use crate::app::{Lane, Shared};
use crate::lanes;
use axum::Json;
use serde_json::{json, Value};

pub(super) fn reply(app: &Shared, event: &str, l: &Lane, p: &Value) -> Json<Value> {
    match event {
        "PostToolUse" => live_messages(app, l),
        // In a lane the rules (with the network sandbox) are the barrier: an allowed call needs no permission prompt, which a headless agent could never answer.
        "PreToolUse" => Json(
            json!({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "allow",
            "permissionDecisionReason": "allowed by Tirith's rules"}}),
        ),
        "Stop" => stop(app, l, p),
        _ => Json(json!({})),
    }
}

/// The lines waiting for the lane, handed to its live agent.
fn live_messages(app: &Shared, l: &Lane) -> Json<Value> {
    let lines = lanes::drain(&l.id);
    let Some(lines) = (!lines.is_empty()).then_some(lines) else {
        return Json(json!({}));
    };
    let _ = (l.status == "waiting" && !l.resume.is_empty()).then(|| {
        app.update_lane(&l.id, |x| {
            (x.status, x.resume) = ("running".into(), String::new())
        })
    }); // the answer reached the live agent
    Json(
        json!({"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": lines.join("\n")}}),
    )
}

/// Keeps the agent's last message as the lane's note, and keeps a running lane from ending its turn on a question.
fn stop(app: &Shared, l: &Lane, p: &Value) -> Json<Value> {
    let last = s(p, "last_assistant_message");
    drop(
        (!last.trim().is_empty())
            .then(|| app.update_lane(&l.id, |x| x.note = last.trim().chars().take(2000).collect())),
    );
    if l.status == "running"
        && p["stop_hook_active"].as_bool() != Some(true)
        && last.trim().ends_with('?')
    {
        return Json(json!({"decision": "block", "reason":
            "Do not end with a question: decide yourself. If you truly need the manager, run: tirith ask \"<question>\" and end your turn."}));
    }
    Json(json!({}))
}
