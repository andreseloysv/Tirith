//! What a run of an experiment is to the rest of Tirith while it is on its way.
use super::seat::{live, seated, SEATS};
use crate::app::App;
use std::path::{Path, PathBuf};

/// `path` of a live B run's worktree as the path of the bot's own folder that it stands for, so what a bot may write there is what it may write at home.
pub fn stands_for(path: &Path) -> Option<PathBuf> {
    let seats = SEATS.lock().unwrap();
    let mut live_b = seats.iter().filter(|(_, s)| s.variant == "B");
    live_b.find_map(|(own, s)| path.strip_prefix(&s.tree).ok().map(|rel| own.join(rel)))
}

/// True while a run of an experiment on bot `key` is on its way.
pub fn experimenting(app: &App, key: &str) -> bool {
    seated(&live(app, key))
}
