//! Muse's results. A command still running when its first result came ends only in Muse's own session log (`muse_log.rs`): its `--json` output never carries that end.
use super::capture_fit::text_of;
use super::look_lines::Ended;
use serde_json::Value;

/// What a line says of a call.
pub(super) enum Said {
    Ended(Ended),
    /// It goes on in the background, as this command.
    Waits(String),
}

/// A `tool.result`: the shell's text is its result JSON.
pub(super) fn result(payload: &Value) -> Said {
    let text = text_of(&payload["text"]);
    let ran: Value = serde_json::from_str(&text).unwrap_or_default();
    let facts = &payload["correlation_facts"];
    let tool = facts["tool_name"].as_str().unwrap_or("tool");
    let what: String = ran["command"].as_str().unwrap_or(tool).into();
    if ran["execution_state"] == "background_running" {
        return Said::Waits(what);
    }
    let failed = facts["outcome"] == "failure"
        || payload["is_error"] == true
        || ran["terminal_status"] == "failed"
        || ran["exit_code"].as_i64().is_some_and(|code| code != 0);
    let output = if ran["command"].is_string() {
        text_of(&ran["output"])
    } else {
        text
    };
    Said::Ended(Ended::of(what, failed, &output))
}

/// A record of the session log: the end of a command that went on in the background (its first result is chunk 1).
pub(super) fn record(payload: &Value) -> Option<Ended> {
    let event = &payload["event"];
    let chunk = event["chunk"]
        .as_str()
        .filter(|_| event["kind"] == "output")?;
    let ran: Value = serde_json::from_str(chunk).ok()?;
    let later = ran["chunk_id"]
        .as_str()
        .is_some_and(|id| !id.ends_with("-1"));
    let what = ran["command"].as_str().filter(|_| later)?;
    let status = ran["terminal_status"].as_str()?;
    let failed = status != "completed" || ran["exit_code"].as_i64().is_some_and(|c| c != 0);
    Some(Ended::of(what.into(), failed, &text_of(&ran["output"])))
}
