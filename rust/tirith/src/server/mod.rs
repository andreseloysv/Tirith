//! HTTP on 127.0.0.1: the page, its JSON API, the change stream (SSE) and the agents' hooks, behind the gate that knows each caller by its token.
mod admit;
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
mod identify;
mod origin;
mod page;
mod peer;
mod person;
mod plan;
mod policy;
mod public;
mod refused;
mod router;
mod routes;
mod scope;
mod settings;
mod state;
mod web;
mod who;

pub use caller::Caller;
pub use door::door;
#[cfg(target_os = "macos")]
pub use hook_door::rules_door;
pub use public::{public_https, url as public_url};
pub use refused::NO_CALLER;
pub use router::router;
pub use who::Who;

pub fn s(v: &serde_json::Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").to_string()
}

#[cfg(test)]
use {hook::hook, origin::origin_host_ok, web::web_file};

#[cfg(test)]
#[path = "../../checks/api.rs"]
mod api_tests;
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
#[path = "../../checks/as_owner.rs"]
pub mod testing;
#[cfg(test)]
#[path = "../../checks/server.rs"]
mod tests;
