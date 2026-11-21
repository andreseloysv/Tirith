//! The tool calls a line of any agent's output makes, the short line the chat shows for each, and the skills they read.
use serde_json::{json, Value};
use std::sync::LazyLock;

/// One tool call: its name, the short line the chat shows, and its input.
pub struct Call {
    pub name: String,
    pub summary: String,
    pub input: Value,
}

/// The tool calls of one line: Anthropic's `tool_use` blocks, Muse's `tool.result`, Codex's started commands.
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
        "item.started" if msg["item"]["type"] == "command_execution" => {
            let cmd = msg["item"]["command"].as_str().unwrap_or("command");
            vec![Call {
                name: "command".into(),
                summary: cmd.into(),
                input: json!({"command": cmd}),
            }]
        }
        _ => vec![],
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

static SKILL_MD: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"skills/([A-Za-z0-9_.:-]+)/SKILL\.md").unwrap());

/// The skill a call read: Anthropic's `Skill` tool, Muse's `read_skill`, or any tool whose input names `skills/<name>/SKILL.md`.
pub fn skill_of(call: &Call) -> Option<String> {
    let named = |v: &Value| {
        ["skill", "name"]
            .iter()
            .find_map(|k| v[*k].as_str().map(String::from))
    };
    let by_tool = match call.name.as_str() {
        "Skill" => named(&call.input),
        "read_skill" => [&call.input, &call.input["input"], &call.input["arguments"]]
            .into_iter()
            .find_map(named),
        _ => None,
    };
    by_tool.or_else(|| {
        SKILL_MD
            .captures(&call.input.to_string())
            .map(|c| c[1].to_string())
    })
}
