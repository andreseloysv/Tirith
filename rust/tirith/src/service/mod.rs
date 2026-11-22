//! `tirith service`: the daemon as the user's own service (a LaunchAgent on macOS, a systemd user unit on Linux), so it starts by itself and comes back when it ends.
mod cli;
mod launchd;
mod lifecycle;
mod manager;
mod plist;
mod run;
mod spec;
mod status;
mod systemd;
mod unit;

pub use cli::cli;

#[cfg(test)]
#[path = "../../checks/service.rs"]
mod tests;
