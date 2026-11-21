//! The folder a project is added from.
use crate::config::{expand, home_dir, Project};
use std::path::{Path, PathBuf};

/// The real folder behind `raw` (a full path or `~/…`), or why it cannot be a project; a project's own folder passes.
pub(super) fn folder(raw: &str, projects: &[Project], tirith: &Path) -> Result<PathBuf, String> {
    let dir = existing(raw)?;
    match super::refusal::why_not(&dir, projects, tirith) {
        Some(w) => Err(format!(
            "{} {w}: pick a project's own folder",
            dir.display()
        )),
        None => Ok(dir),
    }
}

/// The folder `raw` names, as it really is on disk, or why there is none.
fn existing(raw: &str) -> Result<PathBuf, String> {
    let path = if raw.trim() == "~" {
        home_dir()
    } else {
        expand(raw.trim())
    };
    let missing = if path.is_absolute() {
        format!("there is no folder {raw:?}")
    } else {
        format!("{raw:?} is not the full path of a folder (/Users/… or ~/…)")
    };
    path.canonicalize()
        .ok()
        .filter(|d| path.is_absolute() && d.is_dir())
        .ok_or(missing)
}
