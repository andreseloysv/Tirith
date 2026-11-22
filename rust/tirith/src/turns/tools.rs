//! The tool calls a line of any agent's output makes, and the short line the chat shows for each.
use serde_json::Value;

/// One tool call: its name, the short line the chat shows, and its input.
pub struct Call {
    pub name: String,
    pub summary: String,
    pub input: Value,
}

/// The tool calls of one line: Anthropic's `tool_use` blocks, Muse's `tool.result`, Codex's and Copilot's calls (`exec_items.rs`).
pub fn calls(msg: &Value) -> Vec<Call> {
    let kind = msg["payload_type"]
        .as_str()
        .or_else(|| msg["type"].as_str());
    match kind.unwrap_or("") {
        "assistant" => msg["message"]["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["type"] == "tool_use")
            .map(|b| {
                let name = b["name"].as_str().unwrap_or("").to_string();
                let summary = crate::rules::summary(&name, &b["input"]);
                let input = b["input"].clone();
                Call {
                    name,
                    summary,
                    input,
                }
            })
            .collect(),
        "tool.result" => vec![muse(&msg["payload"])],
        _ => super::exec_items::started(msg)
            .map(|(_, name, input)| {
                let command = input["command"].as_str().map(String::from);
                Call {
                    summary: command.unwrap_or_else(|| crate::rules::summary(&name, &input)),
                    name,
                    input,
                }
            })
            .into_iter()
            .collect(),
    }
}

/// A Muse tool result: its tool's name, and the command it ran (its result text is its result JSON) as its line.
fn muse(payload: &Value) -> Call {
    let name = payload["correlation_facts"]["tool_name"].as_str();
    let name = name.unwrap_or("tool").to_string();
    let ran = payload["text"]
        .as_str()
        .and_then(|s| serde_json::from_str::<Value>(s).ok())
        .and_then(|v| v["command"].as_str().map(String::from));
    Call {
        summary: ran.unwrap_or_else(|| name.clone()),
        name,
        input: payload.clone(),
    }
}
