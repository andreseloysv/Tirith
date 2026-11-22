//! `tirith hook`: the command-hook bridge for agents without HTTP hooks.
use crate::client::{call_with, hook_caller};
use serde_json::{json, Value};
use std::{io::Read, time::Duration};

/// The command-hook bridge for agents without HTTP hooks: never breaks the agent (any failure lets the call through); a refusal is exit 2 with the reason on stderr, and for Copilot its JSON alone.
pub(super) fn hook(event: String, home: Option<String>, agent: String) -> Result<(), String> {
    if let Some(home) = home.filter(|h| !h.is_empty()) {
        std::env::set_var("TIRITH_HOME", home); // an agent may hand its hooks none of the variables it was given (Muse)
    }
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);
    let Ok(body) = call_with(
        "POST",
        &format!("/hook/{event}"),
        input,
        &hook_caller(),
        Duration::from_secs(5),
    ) else {
        return Ok(());
    };
    let v: Value = serde_json::from_str(&body).unwrap_or(json!({}));
    let deny = v["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .filter(|_| v["hookSpecificOutput"]["permissionDecision"] == "deny");
    let block = v["reason"].as_str().filter(|_| v["decision"] == "block");
    if let Some(why) = deny.or(block) {
        refuse(why, deny.is_some(), &v, &agent);
    }
    let _ = v["hookSpecificOutput"]
        .is_object()
        .then(|| println!("{body}")); // an allow decision or context for the agent
    Ok(())
}

/// A refusal said both ways at once: exit 2 with the reason on stderr (Codex, Muse), and as JSON on stdout. Copilot gets the JSON with exit 0: on exit 2 it drops the reason.
fn refuse(why: &str, denied: bool, body: &Value, agent: &str) -> ! {
    let said = if denied {
        json!({"permissionDecision": "deny", "permissionDecisionReason": why,
            "hookSpecificOutput": body["hookSpecificOutput"]})
    } else {
        body.clone()
    };
    println!("{said}");
    if agent == "copilot" {
        std::process::exit(0);
    }
    eprintln!("{why}");
    std::process::exit(2);
}
