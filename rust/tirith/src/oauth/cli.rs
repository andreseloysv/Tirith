//! `neobot connector login <id> --bot <bot>`: the owner's, in their own terminal. The daemon gives the sign-in's address, the owner's browser opens it, and this waits until the daemon's callback took the code (ten minutes at most). No token ever comes back here.
use crate::client::{call_json, Flag};
use serde_json::json;
use std::time::{Duration, Instant};

const USE: &str = "usage: neobot connector login <id> --bot <bot> (the owner signs a bot in to a remote connector)";

pub fn cli(pos: &[&str], flag: Flag) -> Option<Result<(), String>> {
    let ["connector", rest @ ..] = pos else {
        return None;
    };
    let bot = flag("--bot").filter(|b| !b.is_empty());
    Some(match (rest, bot) {
        (["login", id], Some(bot)) if !crate::setup::from_agent() => login(id, &bot),
        (["login", _], Some(_)) => Err(
            "neobot connector login is the owner's: run it in your own terminal, not from an agent"
                .into(),
        ),
        _ => Err(USE.into()),
    })
}

fn login(id: &str, bot: &str) -> Result<(), String> {
    let path = format!("/api/connectors/{id}/login");
    let begun = call_json("POST", &path, Some(json!({"bot": bot})))?;
    let (url, ticket) = (begun["url"].as_str(), begun["ticket"].as_str());
    let (Some(url), Some(ticket)) = (url, ticket) else {
        return Err("the daemon gave no sign-in address".into());
    };
    match opened(url) {
        true => println!("sign in to {id} for {bot} in your browser (it opened)"),
        false => println!("open this to sign in to {id} for {bot}:\n{url}"),
    }
    let since = Instant::now();
    while since.elapsed() < Duration::from_secs(600) {
        std::thread::sleep(Duration::from_secs(1));
        let said = call_json("GET", &format!("/api/connectors/logins/{ticket}"), None)?;
        match said["status"].as_str() {
            Some("done") => {
                println!("{id} is signed in for {bot}");
                return Ok(());
            }
            Some("failed") => {
                return Err(format!(
                    "the sign-in failed: {}",
                    said["why"].as_str().unwrap_or("")
                ))
            }
            _ => {}
        }
    }
    Err("no sign-in came within ten minutes: run it again".into())
}

fn opened(url: &str) -> bool {
    std::env::var_os("SSH_CONNECTION").is_none() && crate::os::open(url.as_ref())
}
