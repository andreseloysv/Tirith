//! `/status`: how NeoBot is doing, read from the daemon's own data, so it answers when no model can.
mod command;
mod error;
mod lanes;
mod routines;
mod span;
pub use command::{asked, command};
#[cfg(test)]
#[path = "../../checks/status_runs.rs"]
mod run_tests;
#[cfg(test)]
#[path = "../../checks/status.rs"]
mod tests;
