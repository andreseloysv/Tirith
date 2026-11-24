//! NeoBot's hook in an agent's hooks: its command, and telling it from everyone else's.
use regex::Regex;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::LazyLock;

/// The events NeoBot hears.
pub const EVENTS: [&str; 6] = [
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "SubagentStop",
    "Stop",
];

static OURS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(?:'[^']*[/\\](?:tirith|neobot)(?:\.exe)?'|(?:\S*[/\\])?(?:tirith|neobot)(?:\.exe)?)\s+hook(?:\s|$)"#)
        .unwrap()
});

/// `word` as one word of a shell (on Windows `:`, a drive letter, needs no quotes).
fn word(text: &str) -> String {
    let safe =
        |c: char| c.is_ascii_alphanumeric() || "/._-".contains(c) || (cfg!(windows) && c == ':');
    if !text.is_empty() && text.chars().all(safe) {
        text.to_string()
    } else {
        format!("'{}'", text.replace('\'', r"'\''"))
    }
}

/// A path as a shell word: with forward slashes on Windows, which Git's shell and the agents' hook runners all take.
fn path_word(path: &Path) -> String {
    let text = path.display().to_string();
    word(&if cfg!(windows) {
        text.replace('\\', "/")
    } else {
        text
    })
}

/// The hook's command. `home` is named when it is not the usual one: an agent may hand its hooks none of its variables.
pub fn command(exe: &Path, home: Option<&Path>, alias: &str, event: &str) -> String {
    let mut line = format!("{} hook --agent {alias} --event {event}", path_word(exe));
    if let Some(home) = home {
        line += &format!(" --home {}", path_word(home));
    }
    line
}

/// True for one hook of NeoBot's, as a command or over HTTP.
pub fn ours(hook: &Value) -> bool {
    let line = hook["command"].as_str().or(hook["bash"].as_str());
    let url = hook["url"].as_str().unwrap_or("");
    line.is_some_and(|l| OURS.is_match(l.trim()))
        || (url.starts_with("http://127.0.0.1:") && url.contains("/hook/"))
}

/// An event's hook groups without NeoBot's own hooks; a group left empty goes.
pub fn others(groups: &Value) -> Vec<Value> {
    let groups = groups.as_array().cloned().unwrap_or_default();
    groups
        .into_iter()
        .filter_map(|mut g| {
            if g["hooks"].is_array() {
                let kept: Vec<Value> = g["hooks"]
                    .as_array()?
                    .iter()
                    .filter(|h| !ours(h))
                    .cloned()
                    .collect();
                g["hooks"] = json!(kept);
                (!kept.is_empty()).then_some(g)
            } else {
                (!ours(&g)).then_some(g) // a flat list (Copilot's)
            }
        })
        .collect()
}

/// Whether a settings file's text has NeoBot's hook before tool calls, the one the rules need.
pub fn heard(settings: &str) -> bool {
    let all: Value = serde_json::from_str(settings).unwrap_or_default();
    let groups = all["hooks"]["PreToolUse"].as_array();
    groups
        .into_iter()
        .flatten()
        .any(|g| match g["hooks"].as_array() {
            Some(list) => list.iter().any(ours),
            None => ours(g),
        })
}

/// True when the agent's settings file holds NeoBot's hook.
pub(super) fn hooked(cx: &super::context::Context, known: &super::catalog::Known) -> bool {
    cx.settings(known)
        .and_then(|f| std::fs::read_to_string(f).ok())
        .is_some_and(|t| heard(&t))
}
