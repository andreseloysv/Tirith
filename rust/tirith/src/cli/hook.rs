//! `tirith hook`: the command-hook bridge for agents without HTTP hooks.
use crate::client::call_with;
use serde_json::{json, Value};
use std::{io::Read, time::Duration};

/// The command-hook bridge for agents without HTTP hooks: never breaks the agent (any failure lets the call through); a refusal is exit 2 with the reason on stderr, which every supported agent understands.
pub(super) fn hook(event: String) -> Result<(), String> {
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);
    let headers = [
        (
            "X-Tirith-Lane",
            std::env::var("TIRITH_LANE").unwrap_or_default(),
        ),
        (
            "X-Tirith-Role",
            std::env::var("TIRITH_ROLE").unwrap_or_default(),
        ),
    ];
    let Ok(body) = call_with(
        "POST",
        &format!("/hook/{event}"),
        input,
        &headers,
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
        eprintln!("{why}");
        std::process::exit(2);
    }
    let _ = v["hookSpecificOutput"]
        .is_object()
        .then(|| println!("{body}")); // an allow decision or context for the agent
    Ok(())
}
