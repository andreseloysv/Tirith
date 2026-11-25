//! Which command runs for the words given.
use super::{list::list, status::status};
use crate::{client::Flag, sandbox::tunnel::tunnel};
use serde_json::json;

/// The owner's own browsers, as the commands name them.
macro_rules! OWN {
    () => {
        "safari" | "chrome" | "edge"
    };
}

/// `neobot net …`, `neobot safari|chrome|edge …` and `neobot browser …` commands; `None` when `pos` is not one of this module's commands.
pub fn cli(pos: &[&str], _flag: Flag) -> Option<Result<(), String>> {
    Some(match pos {
        ["net", "status"] => status(),
        ["net", "list"] => list(),
        ["net", "block", rule] => change("POST", rule, "blocked"),
        ["net", "unblock", rule] => change("DELETE", rule, "unblocked"),
        ["net", "tunnel", host, port] => tunnel(host, port),
        ["net", "allow"] => crate::sandbox::confine::allow(),
        ["net", ..] => Err(
            "neobot net status|list|block <domain|ip|cidr>|unblock <rule>|tunnel <host> <port>|allow"
                .into(),
        ),
        [app @ OWN!(), cmd @ ("open" | "read"), rest @ ..] => {
            own(app, json!({"cmd": cmd, "url": rest.join(" ")}))
        }
        [app @ OWN!(), "click", target, rest @ ..] => own(
            app,
            json!({"cmd": "click", "target": target, "url": rest.join(" ")}),
        ),
        [app @ OWN!(), "type", target, text, rest @ ..] => own(
            app,
            json!({"cmd": "type", "target": target, "text": text, "url": rest.join(" ")}),
        ),
        [app @ OWN!(), ..] => Err(format!(
            "neobot {app} open <url> | read [<url>] | click <target> [<url>] | type <target> <text> [<url>]"
        )),
        ["browser"] => own("", json!({"cmd": "browser"})),
        ["browser", "use", which] => own("", json!({"cmd": "use", "target": which})),
        ["browser", "login", rest @ ..] => own("", json!({"cmd": "login", "url": rest.join(" ")})),
        ["browser", ..] => Err(
            "neobot browser | neobot browser use <neobot or the owner's own browser> | neobot browser login <url>"
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

/// Asks the daemon to act in the owner's own browser `app` (Safari on a Mac; Chrome or Edge on Windows) and says what it answered.
fn own(app: &str, mut body: serde_json::Value) -> Result<(), String> {
    body["app"] = json!(app);
    crate::client::call("POST", "/api/safari", Some(body)).map(|b| println!("{b}"))
}
