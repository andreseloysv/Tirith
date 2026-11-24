//! Why a real folder cannot be a project, the first reason that holds.
use crate::config::{home_dir, Project};
use std::path::{Path, PathBuf};

const TEMP: [&str; 6] = [
    "/tmp",
    "/private/tmp",
    "/var/tmp",
    "/private/var/tmp",
    "/var/folders",
    "/private/var/folders",
];

/// Why `dir` cannot be a project, if it cannot: `tirith` is NeoBot's own folder.
pub(super) fn why_not(dir: &Path, projects: &[Project], tirith: &Path) -> Option<String> {
    temporary(dir)
        .or_else(|| holds_home(dir))
        .or_else(|| in_git(dir))
        .or_else(|| in_tirith(dir, tirith))
        .or_else(|| near_project(dir, projects))
}

fn temporary(dir: &Path) -> Option<String> {
    TEMP.map(PathBuf::from)
        .into_iter()
        .chain([std::env::temp_dir()])
        .map(|t| t.canonicalize().unwrap_or(t))
        .any(|t| dir.starts_with(t))
        .then(|| "is a temporary folder".to_string())
}

fn holds_home(dir: &Path) -> Option<String> {
    home_dir()
        .canonicalize()
        .unwrap_or_else(|_| home_dir())
        .starts_with(dir)
        .then(|| "holds your home folder".into())
}

fn in_git(dir: &Path) -> Option<String> {
    (dir.components().any(|c| c.as_os_str() == ".git") || dir.join(".git").is_file())
        .then(|| "is a git worktree or inside a .git folder".into())
}

fn in_tirith(dir: &Path, tirith: &Path) -> Option<String> {
    dir.starts_with(tirith.canonicalize().unwrap_or_else(|_| tirith.into()))
        .then(|| "is NeoBot's own folder (the agents' copies of the projects live there)".into())
}

/// Another project's folder holds `dir`, or `dir` holds one.
fn near_project(dir: &Path, projects: &[Project]) -> Option<String> {
    projects
        .iter()
        .filter(|p| !p.repo.is_empty() && p.repo() != dir)
        .find_map(|p| {
            dir.starts_with(p.repo())
                .then(|| named(p, "is inside the project"))
                .or_else(|| {
                    p.repo()
                        .starts_with(dir)
                        .then(|| named(p, "holds the project"))
                })
        })
}

fn named(p: &Project, what: &str) -> String {
    format!("{what} {}", if p.name.is_empty() { &p.id } else { &p.name })
}
