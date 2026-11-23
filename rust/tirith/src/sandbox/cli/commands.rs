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
        ["net", "allow"] => crate::sandbox::confine::allow(),
        ["net", ..] => Err(
            "tirith net status|list|block <domain|ip|cidr>|unblock <rule>|tunnel <host> <port>|allow"
                .into(),
        ),
        ["safari", cmd @ ("open" | "read"), rest @ ..] => safari(json!({"cmd": cmd, "url": rest.join(" ")})),
        ["safari", "click", target, rest @ ..] => {
            safari(json!({"cmd": "click", "target": target, "url": rest.join(" ")}))
        }
        ["safari", "type", target, text, rest @ ..] => safari(
            json!({"cmd": "type", "target": target, "text": text, "url": rest.join(" ")}),
        ),
        ["safari", ..] => Err(
            "tirith safari open <url> | read [<url>] | click <target> [<url>] | type <target> <text> [<url>]"
                .into(),
        ),
        _ => return None,
    })
}

/// Blocks (POST) or unblocks (DELETE) a rule and says which rule it was.
fn change(method: &str, rule: &str, done: &str) -> Result<(), String> {
    crate::client::call_json(method, "/api/net/blocks", Some(json!({"rule": rule})))
        .map(|v| println!("{done} {}", v["rule"].as_str().unwrap_or("")))
}

/// Asks the daemon to drive the owner's Safari and says what it answered.
fn safari(body: serde_json::Value) -> Result<(), String> {
    crate::client::call("POST", "/api/safari", Some(body)).map(|b| println!("{b}"))
}
