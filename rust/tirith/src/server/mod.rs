//! HTTP on 127.0.0.1: the page, its JSON API, the change stream (SSE) and the agents' hooks.
mod caller;
mod chat;
mod door;
mod hook;
#[cfg(any(test, target_os = "macos"))]
mod hook_door;
mod hook_find;
mod hook_lane;
mod hook_reply;
mod hook_rules;
mod origin;
mod peer;
mod plan;
mod router;
mod routes;
mod settings;
mod state;
mod web;

pub use caller::{agent, agent_name, Caller};
pub use door::door;
#[cfg(target_os = "macos")]
pub use hook_door::rules_door;
pub use router::router;

pub fn s(v: &serde_json::Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").to_string()
}

#[cfg(test)]
use {hook::hook, origin::origin_host_ok, web::web_file};

#[cfg(test)]
#[path = "../../checks/caller.rs"]
mod caller_tests;
#[cfg(test)]
#[path = "../../checks/door.rs"]
mod door_tests;
#[cfg(test)]
#[path = "../../checks/hook_door.rs"]
mod hook_door_tests;
#[cfg(test)]
#[path = "../../checks/server.rs"]
mod tests;
