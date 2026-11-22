//! The tool calls in the exec agents' lines (Codex, Copilot, Muse), under the names those agents' hooks use.
use super::shell::inner;
use serde_json::{json, Value};

/// A tool call that starts on this line: its id, its hook name and its input.
pub(super) fn started(msg: &Value) -> Option<(Value, String, Value)> {
    let (item, data) = (&msg["item"], &msg["data"]);
    let id = item["id"].clone();
    let kind = |k: &str| item["type"] == k;
    match msg["type"].as_str().unwrap_or("") {
        "item.started" if kind("command_execution") => {
            let command = inner(item["command"].as_str().unwrap_or("command"));
            Some((id, "Bash".into(), json!({"command": command})))
        }
        "item.started" if kind("mcp_tool_call") => {
            let part = |k: &str| item[k].as_str().unwrap_or("").to_string();
            let name = format!("mcp__{}__{}", part("server"), part("tool"));
            Some((id, name, item["arguments"].clone()))
        }
        // these two only say that they ended
        "item.completed" if kind("file_change") => {
            let files: Vec<&str> = item["changes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|c| c["path"].as_str())
                .collect();
            Some((id, "Edit".into(), json!({"path": files.join(", ")})))
        }
        "item.completed" if kind("web_search") => {
            Some((id, "WebSearch".into(), json!({"query": item["query"]})))
        }
        "tool.execution_start" => Some((
            data["toolCallId"].clone(),
            hook_name(data["toolName"].as_str().unwrap_or("tool")).into(),
            data["arguments"].clone(),
        )),
        _ => None,
    }
}

/// The result on this line of the call with that id.
pub(super) fn ended(msg: &Value) -> Option<(Value, String)> {
    let (item, data) = (&msg["item"], &msg["data"]);
    let text = |v: &Value| super::capture_fit::text_of(v);
    match msg["type"].as_str().unwrap_or("") {
        "item.completed" if item["type"] == "command_execution" => {
            Some((item["id"].clone(), text(&item["aggregated_output"])))
        }
        "item.completed" if item["type"] == "mcp_tool_call" => {
            Some((item["id"].clone(), text(&item["result"]["content"])))
        }
        "tool.execution_complete" => {
            let said = text(&data["result"]["content"]);
            let why = || text(&data["error"]["message"]);
            let said = if said.is_empty() { why() } else { said };
            Some((data["toolCallId"].clone(), said))
        }
        _ => None,
    }
}

/// Muse's `tool.result`: the call, its input the command it ran (its result's text is that result's JSON), and the result.
pub(super) fn muse(payload: &Value) -> (Value, String) {
    let text = super::capture_fit::text_of(&payload["text"]);
    let ran = serde_json::from_str::<Value>(&text).ok();
    let ran = ran.and_then(|v| v["command"].as_str().map(String::from));
    let input = ran.map_or(json!({}), |c| json!({"command": c}));
    let tool = &payload["correlation_facts"]["tool_name"];
    (json!({"tool": tool, "input": input}), text)
}

/// Copilot's hooks (registered under the event names the other agents use) name its tools as they do.
fn hook_name(tool: &str) -> &str {
    match tool {
        "bash" => "Bash",
        "create" => "Write",
        "edit" | "apply_patch" => "Edit",
        "view" => "Read",
        "skill" => "Skill",
        "web_fetch" => "WebFetch",
        other => other,
    }
}
