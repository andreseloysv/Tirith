//! Which tool calls of a replay read outside (the web, a browser, `tirith safari` or `tirith teach do`, shell commands that use the network) and which would reach the owner (`tirith say`, `ask`, `ask-owner`, `chat`).
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

/// A command that uses the network, at the start of a command or after a separator.
static NET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[\s;&|(`$/])(?:curl|wget|https?|ssh|scp|sftp|rsync|nc|ncat|telnet|ftp|ping|dig|nslookup|gh|osascript|tirith\s+(?:safari|teach)|git\s+(?:fetch|pull|clone|push|ls-remote|remote\s+update)|open\s+https?:)(?:\s|$|;|\|)").unwrap()
});
/// A `tirith` command that speaks to the owner or opens a card.
static OWNER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\btirith\s+(?:say|ask|ask-owner|chat)\b").unwrap());

/// The command of a shell call.
fn command(tool: &str, input: &Value) -> Option<String> {
    let cmd = input["command"].as_str().or(input["cmd"].as_str());
    let shell = matches!(tool, "Bash" | "shell" | "exec_command" | "run_command");
    cmd.filter(|_| shell).map(String::from)
}

/// True for a call (under the rules' names) that reads outside the bot's folder.
pub fn outside(tool: &str, input: &Value) -> bool {
    let web = matches!(
        tool,
        "WebFetch" | "WebSearch" | "web_fetch" | "web_search" | "fetch" | "search"
    );
    let lower = tool.to_lowercase();
    web || tool.starts_with("mcp__")
        || lower.contains("browser")
        || command(tool, input).is_some_and(|c| NET.is_match(&c))
}

/// True for a call that would reach the owner: nothing of a replay does.
pub fn to_owner(tool: &str, input: &Value) -> bool {
    command(tool, input).is_some_and(|c| OWNER.is_match(&c))
}
