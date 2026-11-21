//! A normalized tool call as the rules read it.
use serde_json::Value;

pub(super) struct Call<'a> {
    pub tool: &'a str,
    /// A shell call's command ("" for any other call).
    pub cmd: &'a str,
    /// Every file the call names (`file_paths`, see `normalize`).
    pub files: Vec<&'a str>,
    pub edit: bool,
    pub bash: bool,
}

impl<'a> Call<'a> {
    pub fn new(tool: &'a str, input: &'a Value) -> Call<'a> {
        Call {
            tool,
            cmd: input["command"].as_str().unwrap_or(""),
            files: input["file_paths"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str())
                .collect(),
            edit: super::EDIT_TOOLS.contains(&tool),
            bash: tool == "Bash",
        }
    }
}
