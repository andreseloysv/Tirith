//! The command line of one Copilot turn, and what its stream says of the answer.
use serde_json::Value;
use std::path::Path;

/// The argv of one Copilot turn: the message comes on stdin, its usage goes to `usage`. It runs unattended inside NeoBot's sandbox and rules, as Codex and Muse do, so nothing asks; its folder is NeoBot's, so its skills load.
pub(super) fn copilot_argv(
    command: &str,
    read_only: bool,
    model: (Option<&str>, Option<&str>),
    resume: Option<&str>,
    usage: &Path,
    extra: &[String],
) -> Vec<String> {
    let mut a: Vec<String> = [
        command,
        "--output-format",
        "json",
        "--no-ask-user",
        "--no-auto-update",
        "--allow-all",
        "--log-level",
        "none",
        "--usage-output-file",
    ]
    .map(String::from)
    .to_vec();
    a.push(usage.display().to_string());
    a.extend(resume.is_none().then(|| "--trust-folder".to_string())); // a new session's only: refused on a resume
    a.extend(read_only.then(|| "--deny-tool=write".to_string()));
    let (model, effort) = (model.0, effort_of(model.0, model.1));
    for (flag, value) in [("--model", model), ("--reasoning-effort", effort)] {
        a.extend(
            value
                .into_iter()
                .flat_map(|v| [flag.to_string(), v.to_string()]),
        );
    }
    a.extend(resume.map(|id| format!("--resume={id}")));
    a.extend(extra.iter().cloned()); // a bot's connector gateway
    a
}

/// The effort Copilot can be told: only beside a named model. Its `auto` (all a free plan has, and what runs when none is named) refuses one, and the turn with it.
pub fn effort_of<'a>(model: Option<&str>, effort: Option<&'a str>) -> Option<&'a str> {
    effort.filter(|_| model.is_some_and(|m| !m.eq_ignore_ascii_case("auto")))
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
