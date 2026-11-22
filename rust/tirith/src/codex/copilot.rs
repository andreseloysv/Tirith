//! The command line of one Copilot turn, and what its stream says of the answer.
use serde_json::Value;
use std::path::Path;

/// The argv of one Copilot turn: the message comes on stdin, its usage goes to `usage`. It runs unattended inside Tirith's sandbox and rules, as Codex and Muse do, so nothing asks; its folder is Tirith's, so its skills load.
pub(super) fn copilot_argv(
    command: &str,
    read_only: bool,
    model: (Option<&str>, Option<&str>),
    resume: Option<&str>,
    usage: &Path,
) -> Vec<String> {
    let mut a: Vec<String> = [
        command,
        "--output-format",
        "json",
        "--no-ask-user",
        "--no-auto-update",
        "--allow-all",
        "--trust-folder",
        "--usage-output-file",
    ]
    .map(String::from)
    .to_vec();
    a.push(usage.display().to_string());
    a.extend(read_only.then(|| "--deny-tool=write".to_string()));
    let (model, effort) = model;
    for (flag, value) in [("--model", model), ("--reasoning-effort", effort)] {
        a.extend(
            value
                .into_iter()
                .flat_map(|v| [flag.to_string(), v.to_string()]),
        );
    }
    a.extend(resume.map(|id| format!("--resume={id}")));
    a
}

/// The words of a message of the agent itself (a subagent's are no answer).
pub fn said(msg: &Value) -> Option<&str> {
    let data = &msg["data"];
    let own = msg["type"] == "assistant.message"
        && data["parentToolCallId"].is_null()
        && msg["agentId"].is_null();
    data["content"]
        .as_str()
        .filter(|text| own && !text.trim().is_empty())
}
