//! The daemon's shared state: config, SQLite, the lanes in memory and the change feed the page listens to.
mod config;
mod conv;
mod feed;
mod git;
mod lane;
mod lanes;
mod migrate;
mod open;
mod path;
mod process;
mod query;
mod reply;
mod state;
mod store;
mod text;
mod time;
mod worktree;

pub use git::git;
pub use lane::Lane;
pub use path::exe;
pub use process::{alive, kill_group};
pub use reply::{bad, R};
pub use state::{App, Shared};
pub use text::{redact_secrets, tail};
pub use time::{now_iso, now_s};
pub use worktree::add_worktree;

#[cfg(test)]
use {migrate::migrate, path::path_with_cargo, std::path::Path};

#[cfg(test)]
#[path = "../../checks/app.rs"]
mod tests;
