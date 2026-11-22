//! Inside a replay's or a shadow B run's turn: the worktree that is the bot's folder for it and the routine it runs, where the routine's own `routines.toml` is read from.
use std::path::PathBuf;

/// The worktree and routine of the replay or shadow run this turn is in, if any.
pub fn scope() -> Option<(PathBuf, String)> {
    super::replay::run().or_else(super::shadow_open::run)
}
