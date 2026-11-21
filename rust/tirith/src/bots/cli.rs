//! A bot's own commands, in its conversation.
use crate::client::{call, print, Flag};
use serde_json::json;
/// A bot's own commands, in its conversation (`TIRITH_PROJECT` is `bot-<id>`): `tirith routine add|remove|list`.
pub fn cli(pos: &[&str], _: Flag) -> Option<Result<(), String>> {
    let key = std::env::var("TIRITH_PROJECT").unwrap_or_default();
    let path = format!(
        "/api/bots/{}/routines",
        key.strip_prefix("bot-").unwrap_or(&key)
    );
    match pos {
        ["routine", "add", title, days, at, task] => Some(print(call(
            "POST",
            &path,
            Some(json!({"title": title, "days": days, "at": at, "prompt": task})),
        ))),
        ["routine", "remove", title] => Some(print(call(
            "POST",
            &path,
            Some(json!({"title": title, "delete": true})),
        ))),
        ["routine", "list"] => Some(call("GET", &path, None).map(|b| println!("{b}"))),
        _ => None,
    }
}
