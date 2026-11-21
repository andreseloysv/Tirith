//! Which command runs for the words given.
use super::{list::list, status::status};
use crate::{client::Flag, sandbox::tunnel::tunnel};
use serde_json::json;

/// `tirith net …` and `tirith safari …` commands; `None` when `pos` is not one of this module's commands.
pub fn cli(pos: &[&str], _flag: Flag) -> Option<Result<(), String>> {
    Some(match pos {
        ["net", "status"] => status(),
        ["net", "list"] => list(),
        ["net", "block", rule] => change("POST", rule, "blocked"),
        ["net", "unblock", rule] => change("DELETE", rule, "unblocked"),
        ["net", "tunnel", host, port] => tunnel(host, port),
        ["net", ..] => Err(
            "tirith net status|list|block <domain|ip|cidr>|unblock <rule>|tunnel <host> <port>"
                .into(),
        ),
        ["safari", cmd @ ("open" | "read"), rest @ ..] => crate::client::call(
            "POST",
            "/api/safari",
            Some(json!({"cmd": cmd, "url": rest.join(" ")})),
        )
        .map(|b| println!("{b}")),
        ["safari", ..] => Err("tirith safari open <url> | tirith safari read [<url>]".into()),
        _ => return None,
    })
}

/// Blocks (POST) or unblocks (DELETE) a rule and says which rule it was.
fn change(method: &str, rule: &str, done: &str) -> Result<(), String> {
    crate::client::call_json(method, "/api/net/blocks", Some(json!({"rule": rule})))
        .map(|v| println!("{done} {}", v["rule"].as_str().unwrap_or("")))
}
