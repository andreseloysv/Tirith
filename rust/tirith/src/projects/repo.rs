//! The git repository a folder to add is in, if any.
use crate::app::git;
use crate::config::home_dir;
use std::path::{Path, PathBuf};

/// The top of the git repository `dir` is in, when there is one. A repo git cannot read is an error, not a folder to init; a home folder kept in git is no project's repo.
pub(super) async fn git_top(dir: &Path) -> Result<Option<PathBuf>, String> {
    let home = home_dir().canonicalize().unwrap_or_else(|_| home_dir());
    match git(dir, &["rev-parse", "--show-toplevel"]).await {
        Err(e) if dir.join(".git").exists() => Err(e),
        r => Ok(r
            .ok()
            .map(|t| {
                PathBuf::from(&t)
                    .canonicalize()
                    .unwrap_or_else(|_| t.into())
            })
            .filter(|t| !home.starts_with(t))),
    }
}

/// A folder inside a repository is not added: its repository is.
pub(super) fn not_inside(dir: &Path, top: Option<&PathBuf>) -> Result<(), String> {
    top.filter(|t| **t != dir).map_or(Ok(()), |t| {
        Err(format!(
            "{} is inside the git repository {}: add that folder instead",
            dir.display(),
            t.display()
        ))
    })
}
