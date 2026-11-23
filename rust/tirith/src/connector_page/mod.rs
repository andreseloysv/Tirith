//! The Connectors page, the owner's alone: each connector with each bot's status (needs login, ok or error, the why without a secret), the bots granted it, its last use and its real tools marked allowed, approval or denied for a bot, with Login (`oauth/`), Block / Unblock (`access/`) and Remove (its row and every grant). Only the config's tables and the Access APIs say what is granted or blocked; what the gateway last saw (tools, a failed open or sign-in) is kept in kv (docs/DAEMON.md).
mod marks;
mod remove;
mod routes;
mod scrub;
mod seen;
mod state;
mod status;

pub use routes::{owner_only, routes};
pub use seen::{failed, worked};
