//! The short line a tool call shows on the phones.
use serde_json::Value;
/// The short line a tool call shows on the phones: its own summary (a description, a file, a pattern, a url) when it has one, never its raw command; without one, only the tool's name.
pub(super) fn progress_line(tool: &str, input: &Value) -> String {
    let detail = [
        "description",
        "file_path",
        "notebook_path",
        "pattern",
        "url",
        "prompt",
        "skill",
    ]
    .iter()
    .find_map(|k| input[*k].as_str())
    .unwrap_or("");
    let short = detail.lines().next().unwrap_or("").trim();
    Some(short).filter(|s| !s.is_empty()).map_or_else(
        || tool.to_string(),
        |short| {
            format!(
                "{}{short}",
                if tool.is_empty() {
                    String::new()
                } else {
                    format!("{tool}: ")
                }
            )
        },
    )
}
