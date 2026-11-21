//! `tirith room …` commands.
use super::show::{list, show_link, tail};
use crate::client::{call, call_json, Flag};
use serde_json::json;
const USAGE: &str = include_str!("../../assets/usage-room.txt");
/// `tirith room …` commands; `None` when `pos` is not one of this module's commands.
pub fn cli(pos: &[&str], flag: Flag) -> Option<Result<(), String>> {
    let r = match pos {
        ["room", "new"] => new(flag),
        ["room", "forget", key] => call("POST", "/api/rooms/forget", Some(json!({"key_id": key})))
            .map(|_| println!("forgot {key}")),
        ["room", "link", id] => call_json("GET", &format!("/api/rooms/{id}"), None)
            .map(|v| show_link(&v, "link", "Open on my phone")),
        ["room", "qr", id] => call_json("POST", &format!("/api/rooms/{id}/qr"), None)
            .map(|v| show_link(&v, "invite", "Connect another phone (one use)")),
        ["room", "invite", id] => {
            call_json("POST", &format!("/api/rooms/{id}/invite"), None).map(|v| {
                println!(
                    "{}\nThe other Tirith runs: tirith room join <that code>",
                    v["peer_code"].as_str().unwrap_or("")
                )
            })
        }
        ["room", "join", code] => call_json("POST", "/api/rooms/join", Some(json!({"code": code})))
            .map(|v| println!("joined {}", v["id"].as_str().unwrap_or(""))),
        ["room", "leave", id] => {
            call("POST", &format!("/api/rooms/{id}/leave"), None).map(|_| println!("left {id}"))
        }
        ["room", "say", id, text] => call(
            "POST",
            &format!("/api/rooms/{id}/say"),
            Some(json!({"text": text})),
        )
        .map(|_| println!("ok")),
        ["room", "list"] => call_json("GET", "/api/rooms", None).map(|v| list(&v)),
        ["room", "tail", id] => {
            call_json("GET", &format!("/api/rooms/{id}"), None).map(|v| tail(&v))
        }
        ["room", ..] => Err(USAGE.to_string()),
        _ => return None,
    };
    Some(r)
}
fn new(flag: Flag) -> Result<(), String> {
    let project = flag("--project")
        .or_else(|| std::env::var("TIRITH_PROJECT").ok())
        .unwrap_or_default();
    let body = json!({"title": flag("--title").unwrap_or_default(), "project": project});
    call_json("POST", "/api/rooms", Some(body)).map(|v| {
        println!(
            "{}  (open on my phone: tirith room link {0})",
            v["id"].as_str().unwrap_or("")
        );
        show_link(&v, "invite", "Connect a phone (one use)");
    })
}
