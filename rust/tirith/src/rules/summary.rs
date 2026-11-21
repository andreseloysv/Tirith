//! A tool call in one short line, for logs and the chat.
use serde_json::Value;

pub fn summary(tool: &str, input: &Value) -> String {
    let detail = [
        "command",
        "file_path",
        "notebook_path",
        "path",
        "pattern",
        "url",
        "description",
        "prompt",
        "skill",
    ]
    .iter()
    .find_map(|k| input[*k].as_str())
    .unwrap_or("");
    format!(
        "{}{}",
        if tool.is_empty() {
            String::new()
        } else {
            format!("{tool}: ")
        },
        detail
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(160)
            .collect::<String>()
    )
}
