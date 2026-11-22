//! What a Copilot turn says while it runs: its messages in the chat and on the phones, its tools' calls in the chat.
use super::talk::talk;
use super::typing::typing_line;
use crate::app::Shared;
use crate::rooms;
use serde_json::Value;

/// A Copilot event: its session id is kept, its own messages are said as they come, its tool calls shown.
pub(super) fn on_copilot(app: &Shared, project: &str, key: &str, msg: &Value) {
    let data = &msg["data"];
    match msg["type"].as_str().unwrap_or("") {
        "session.start" | "session.resume" => data["sessionId"]
            .as_str()
            .into_iter()
            .for_each(|id| app.set_kv(key, Some(id))),
        "result" => msg["sessionId"]
            .as_str()
            .into_iter()
            .for_each(|id| app.set_kv(key, Some(id))),
        "assistant.message" => {
            let Some(text) = crate::codex::said(msg) else {
                return;
            };
            app.chat("manager", text, project);
            let _ = talk(project, |t| t.phones).then(|| rooms::to_phones(app, project, text));
        }
        "tool.execution_start" => {
            let name = data["toolName"].as_str().unwrap_or("tool");
            app.chat(
                "tool",
                &crate::rules::summary(name, &data["arguments"]),
                project,
            );
            typing_line(project, name);
            // the phones get the tool's name, never its arguments
        }
        _ => {}
    }
}
