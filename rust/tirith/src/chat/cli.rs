//! `neobot chat clear|keep`.
use crate::client::{call_json, Flag};
use crate::server::s;
use serde_json::{json, Value};

/// `neobot chat clear|keep [<the owner's words>]`: on the conversation of the session that runs it (`TIRITH_PROJECT`), or `--project`'s. A lane never clears a chat: only a bot the owner talks with, on the owner's own words (its prompt says so).
pub fn cli(pos: &[&str], flag: Flag) -> Option<Result<(), String>> {
    let conv = || conv(&flag);
    match pos {
        ["chat", "clear"] => Some(conv().and_then(|p| call_json("POST", "/api/chat/clear", Some(json!({"project": p})))).map(|v| println!("chat cleared ({})", s(&v, "project")))),
        ["chat", "keep"] => Some(conv().and_then(|p| say(call_json("GET", &format!("/api/chat/keep?project={p}"), None), "the chat lives"))),
        ["chat", "keep", words @ ..] => Some(conv().and_then(|p| say(call_json("POST", "/api/chat/keep", Some(json!({"project": p, "keep": words.join(" ")}))), "the chat now lives"))),
        ["chat", ..] => Some(Err("use: neobot chat clear | neobot chat keep [<time, as 1h, 24 h, 2 days, a week, forever>]".into())),
        _ => None,
    }
}

/// The conversation the command is on, refused in a lane.
fn conv(flag: &Flag) -> Result<String, String> {
    let lane = std::env::var("TIRITH_LANE").is_ok_and(|l| !l.is_empty());
    let Some(()) = (!lane).then_some(()) else {
        return Err(
            "a lane does not clear chats: only the bot the owner asked, in its own conversation"
                .to_string(),
        );
    };
    Ok(flag("--project")
        .or_else(|| std::env::var("TIRITH_PROJECT").ok())
        .unwrap_or_default())
}

/// Prints a conversation's longest life, «siempre» when it has none.
fn say(r: Result<Value, String>, what: &str) -> Result<(), String> {
    r.map(|v| {
        println!(
            "{what} ({}): {}",
            s(&v, "project"),
            Some(s(&v, "keep"))
                .filter(|k| !k.is_empty())
                .unwrap_or_else(|| "forever".into())
        )
    })
}
