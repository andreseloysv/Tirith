//! The guide's talk with the daemon of this home.
use crate::client::{call_json, call_with};
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, Instant};

/// The daemon that answers on this home's port, as it says who it is (an older one, and another home's, which refuses this one's token, say nothing of themselves: null).
pub fn answering() -> Option<Value> {
    match call_json("GET", "/api/setup", None) {
        Ok(v) => Some(v),
        Err(why) if why.starts_with(crate::server::NO_CALLER) => Some(Value::Null),
        Err(_) => call_json("GET", "/api/state", None)
            .ok()
            .map(|_| Value::Null),
    }
}

/// Whether that daemon is `home`'s.
pub fn is_of(said: &Value, home: &Path) -> bool {
    let home = home.canonicalize().unwrap_or_else(|_| home.to_path_buf());
    said["home"].as_str().is_some_and(|h| Path::new(h) == home)
}

/// Waits until this home's daemon answers.
pub fn up(home: &Path) -> Result<(), String> {
    wait(|| answering().is_some_and(|v| is_of(&v, home)))
        .then_some(())
        .ok_or_else(|| {
            format!(
                "the daemon did not come up: see {}/logs/tirithd.log",
                home.display()
            )
        })
}

/// Waits until nothing answers.
pub fn down() -> Result<(), String> {
    wait(|| answering().is_none())
        .then_some(())
        .ok_or_else(|| "the daemon still answers".to_string())
}

fn wait(done: impl Fn() -> bool) -> bool {
    let until = Instant::now() + Duration::from_secs(20);
    while Instant::now() < until {
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    false
}

/// The connection check of one agent (a real turn: up to 10 minutes).
pub fn check(alias: &str) -> Result<Value, String> {
    let body = call_with(
        "POST",
        &format!("/api/setup/check/{alias}"),
        "{}".into(),
        &crate::client::caller(),
        Duration::from_secs(620),
    )?;
    serde_json::from_str(&body).map_err(|e| e.to_string())
}
