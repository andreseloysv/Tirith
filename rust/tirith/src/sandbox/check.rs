//! The block list for agent sessions NOT started by OpenBot (the owner's own sessions), checked in the hook.
use super::blocks::{blocked, none};
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

static HOSTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[^a-z0-9_.-])((?:[a-z0-9-]+\.)+[a-z0-9-]+|\[[0-9a-f:.]+\])").unwrap()
});

/// The refusal when a tool call names a blocked destination (a URL or host in a Bash command, a WebFetch url), else `None`.
pub fn check_command(tool: &str, input: &Value) -> Option<String> {
    (!none()).then_some(())?;
    let text = named_text(tool, input)?;
    HOSTS.captures_iter(text).find_map(|c| {
        blocked(&c[1]).map(|rule| {
            format!(
                "{} is blocked by the owner (rule {rule}): OpenBot refuses calls to it. Use another source, or ask the \
         owner to allow it again (tirith net unblock {rule}, or Desbloquear in the Red section of the page).",
                &c[1]
            )
        })
    })
}

/// The text of a tool call that may name a destination (`tirith net …` commands never count).
fn named_text<'a>(tool: &str, input: &'a Value) -> Option<&'a str> {
    match tool {
        "Bash" => input["command"]
            .as_str()
            .filter(|c| !c.contains("tirith net ")),
        "WebFetch" => input["url"].as_str(),
        _ => None,
    }
}
