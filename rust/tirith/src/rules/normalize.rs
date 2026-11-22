//! Every agent's tool call in the names the rules know.
use serde_json::{json, Value};

pub const EDIT_TOOLS: [&str; 7] = [
    "Edit",
    "Write",
    "MultiEdit",
    "NotebookEdit",
    "edit_file",
    "write_file",
    "apply_patch",
];

/// Every agent's tool call in the names the rules know, translated in this one place before `check` and the sandbox audit (`tags`, `check_command`): a shell is `Bash` (also `bash` and `monitor`, the text in `command`); an edit is one of the four edit tools above (also `edit_file`/`write_file`, the file in `path`, and `apply_patch`, whose files are its patch's `*** … File:` and `*** Move to:` lines). The call keeps its input, with its files — every one of them, relative ones taken against `cwd` — in `file_path` (the first) and `file_paths` (all). Any other call keeps its name.
pub fn normalize(tool: &str, input: &Value, cwd: &str) -> (String, Value) {
    let at = |p: &str| {
        if std::path::Path::new(p.trim()).is_absolute() || cwd.is_empty() {
            p.trim().to_string()
        } else {
            std::path::Path::new(cwd)
                .join(p.trim())
                .display()
                .to_string()
        }
    };
    let no_path = ["file_path", "notebook_path", "path"]
        .iter()
        .all(|k| input[*k].is_null());
    let files: Vec<String> = if tool == "apply_patch" || (tool == "Edit" && no_path) {
        patch_files(input).into_iter().map(&at).collect()
    } else {
        ["file_path", "notebook_path", "path"]
            .iter()
            .find_map(|k| input[*k].as_str())
            .map(&at)
            .into_iter()
            .collect()
    };
    let name = match tool {
        "bash" | "monitor" => "Bash",
        "write_file" => "Write",
        "edit_file" | "apply_patch" => "Edit",
        t => t,
    };
    let mut v = input.clone();
    if let Some(first) = files.first().filter(|_| v.is_object()) {
        (v["file_path"], v["file_paths"]) = (json!(first), json!(files));
    }
    (name.into(), v)
}

/// The files a patch names on its `*** … File:` and `*** Move to:` lines, as written.
fn patch_files(input: &Value) -> Vec<&str> {
    let heads = [
        "*** Add File:",
        "*** Update File:",
        "*** Delete File:",
        "*** Move to:",
    ];
    input
        .as_str()
        .into_iter()
        .chain(
            input
                .as_object()
                .into_iter()
                .flat_map(|o| o.values().filter_map(|v| v.as_str())),
        )
        .flat_map(|t| t.lines())
        .filter_map(|l| heads.iter().find_map(|h| l.trim_start().strip_prefix(h)))
        .collect()
}
