//! The daemon's HTTP proxy for sandboxed processes (`CONNECT` and plain-HTTP requests): it authenticates the caller, refuses blocked destinations and records each new one.
mod answer;
mod auth;
mod dests;
mod forward;
mod handle;
mod record;
mod request;
mod serve;
mod upstream;
#[cfg(test)]
pub(super) use upstream::own_door;

pub use dests::{destinations, seed};
pub use serve::{serve, state};
