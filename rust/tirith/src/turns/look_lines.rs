//! The pieces of an agent's lines that say how a look ended.
use serde_json::Value;

/// The content blocks of one kind in a message.
pub(super) fn blocks<'a>(msg: &'a Value, kind: &'a str) -> impl Iterator<Item = &'a Value> + 'a {
    let all = msg["message"]["content"].as_array().into_iter().flatten();
    all.filter(move |b| b["type"] == kind)
}

/// A Muse result: its text is the command's result JSON, when it was the shell.
pub(super) fn muse(payload: &Value) -> (String, bool) {
    let text = payload["text"].as_str().unwrap_or("");
    let ran: Value = serde_json::from_str(text).unwrap_or_default();
    let tool = payload["correlation_facts"]["tool_name"]
        .as_str()
        .unwrap_or("tool");
    let code = ["exit_code", "exitCode", "code"]
        .iter()
        .find_map(|k| ran[*k].as_i64());
    let failed = payload["is_error"] == true
        || payload["status"] == "error"
        || payload["outcome"] == "error"
        || code.is_some_and(|c| c != 0);
    (ran["command"].as_str().unwrap_or(tool).into(), failed)
}
