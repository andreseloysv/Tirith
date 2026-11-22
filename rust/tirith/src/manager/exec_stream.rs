//! What an exec turn says while it runs: Muse's stream, Codex's and Copilot's events, in the chat and on the phones.
use super::exec_copilot::on_copilot;
use super::talk::talk;
use super::typing::typing_line;
use crate::app::Shared;
use crate::codex::Exec;
use crate::rooms;
use serde_json::Value;
/// One event of the agent's stream.
pub(super) fn on_event(app: &Shared, project: &str, key: &str, kind: Exec, msg: &Value) {
    match kind {
        Exec::Codex => on_codex(app, project, key, msg),
        Exec::Copilot => on_copilot(app, project, key, msg),
        Exec::Muse => on_muse(app, project, key, msg),
    }
}
/// A Muse event: its session id is kept, and a tool's result shows the command it ran.
fn on_muse(app: &Shared, project: &str, key: &str, msg: &Value) {
    msg["stream"]["id"]
        .as_str()
        .filter(|_| msg["stream"]["kind"] == "session")
        .into_iter()
        .for_each(|id| app.set_kv(key, Some(id)));
    if msg["payload_type"] == "tool.result" {
        // The shell's own result text is its result JSON: the command it ran is the line worth showing.
        let detail = msg["payload"]["text"]
            .as_str()
            .and_then(|s| serde_json::from_str::<Value>(s).ok())
            .and_then(|v| v["command"].as_str().map(String::from));
        app.chat(
            "tool",
            detail.as_deref().unwrap_or_else(|| {
                msg["payload"]["correlation_facts"]["tool_name"]
                    .as_str()
                    .unwrap_or("tool")
            }),
            project,
        );
        typing_line(
            project,
            msg["payload"]["correlation_facts"]["tool_name"]
                .as_str()
                .unwrap_or("tool"),
        );
        // the phones get the tool's name, never its raw command
    }
}
/// A Codex event: its thread id is kept, its messages are said as they come, its commands shown.
fn on_codex(app: &Shared, project: &str, key: &str, msg: &Value) {
    match msg["type"].as_str().unwrap_or("") {
        "thread.started" => msg["thread_id"]
            .as_str()
            .into_iter()
            .for_each(|id| app.set_kv(key, Some(id))),
        "item.completed" if msg["item"]["type"] == "agent_message" => {
            let text = msg["item"]["text"].as_str().unwrap_or("");
            app.chat("manager", text, project);
            let _ = talk(project, |t| t.phones).then(|| rooms::to_phones(app, project, text));
        }
        "item.started" if msg["item"]["type"] == "command_execution" => {
            let cmd = msg["item"]["command"].as_str().unwrap_or("command");
            app.chat("tool", cmd, project);
            typing_line(
                project,
                cmd.split_whitespace()
                    .find(|w| !w.contains('='))
                    .unwrap_or("command"),
            );
            // only the program's name: its arguments may hold anything
        }
        _ => {}
    }
}
