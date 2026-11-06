//! The commit guard: git hooks in every project that refuse a commit whose message, file names or added lines
//! contain one of the owner's forbidden words (STUB: the misc agent replaces the bodies, keeping every signature).

use crate::app::Shared;
use crate::config::Project;
use axum::Router;

/// Ok when the project's guard is installed and intact (or the project opts out); the reason otherwise.
/// The scheduler calls it before it starts a lane of the project.
pub fn intact(_app: &Shared, _project: &Project) -> Result<(), String> {
    Ok(())
}

pub fn start(_app: Shared) {}

pub fn routes() -> Router<Shared> {
    Router::new()
}

/// `tirith guard …` commands.
pub fn cli(_pos: &[&str], _flag: crate::client::Flag) -> Option<Result<(), String>> {
    None
}
