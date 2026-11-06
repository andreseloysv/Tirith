//! agentsmessenger.com rooms (STUB: the rooms agent replaces the bodies, keeping every signature).
//!
//! Public rooms Tirith watches, and private `sealed-` rooms where the owner talks to the manager from a phone,
//! end-to-end encrypted, compatible with `am.mjs` and the agentsmessenger.com chat page.

use crate::app::Shared;
use axum::Router;

/// The manager's answer to a turn that rooms started: posted to each room in `rooms` (sealed in a private room).
pub fn reply(_app: &Shared, _rooms: &[String], _text: &str) {}

pub fn start(_app: Shared) {}

pub fn routes() -> Router<Shared> {
    Router::new()
}

/// `tirith room …` commands; `None` when `pos` is not one of this module's commands.
pub fn cli(_pos: &[&str], _flag: crate::client::Flag) -> Option<Result<(), String>> {
    None
}
