//! What any agent's line says of its turn: its session, its model and the answer's words.
use serde_json::Value;

/// The agent's session (Codex's thread), as it names it.
pub(super) fn session(msg: &Value) -> Option<&str> {
    let muse = msg["stream"]["id"]
        .as_str()
        .filter(|_| msg["stream"]["kind"] == "session");
    let copilot = || {
        msg["data"]["sessionId"]
            .as_str()
            .or(msg["sessionId"].as_str())
    };
    muse.or_else(|| msg["thread_id"].as_str())
        .or_else(|| msg["session_id"].as_str())
        .or_else(copilot)
}

/// The model a line says the turn runs on.
pub(super) fn model(msg: &Value) -> Option<&str> {
    let kind = msg["payload_type"].as_str().or(msg["type"].as_str());
    let model = match kind.unwrap_or("") {
        "system" => &msg["model"],
        "run.model.configured" => &msg["payload"]["model_id"],
        "runtime.session" => &msg["payload"]["event"]["model"],
        "session.auto_mode_resolved" => &msg["data"]["chosenModel"],
        "assistant.message" => &msg["data"]["model"],
        _ => return None,
    };
    model.as_str().filter(|m| !m.is_empty() && *m != "auto")
}

/// The answer's words on a line: Anthropic's text blocks, Codex's and Copilot's messages (Muse's answer is handed over at its end).
pub(super) fn words(msg: &Value) -> Vec<&str> {
    match msg["type"].as_str().unwrap_or("") {
        "assistant" => msg["message"]["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["type"] == "text")
            .map(|b| b["text"].as_str().unwrap_or(""))
            .collect(),
        "item.completed" if msg["item"]["type"] == "agent_message" => {
            vec![msg["item"]["text"].as_str().unwrap_or("")]
        }
        "assistant.message" => crate::codex::said(msg).into_iter().collect(),
        _ => vec![],
    }
}
