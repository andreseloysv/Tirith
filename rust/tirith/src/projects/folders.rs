//! Git repositories that could be added as projects.
use crate::app::Shared;
use crate::config::{home_dir, Project};
use axum::{extract::State, Json};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Git repositories next to the projects or in the home folder that are no project yet, the most recently used first.
pub(super) async fn folders(State(app): State<Shared>) -> Json<Value> {
    let repos: Vec<PathBuf> = app
        .cfg()
        .projects
        .iter()
        .filter(|p| !p.repo.is_empty())
        .map(Project::repo)
        .collect();
    let mut found: Vec<_> = roots(&repos)
        .iter()
        .flat_map(|r| std::fs::read_dir(r).into_iter().flatten().flatten())
        .map(|e| e.path())
        .filter(|d| {
            !repos.contains(d)
                && !d
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        })
        .filter_map(used)
        .collect();
    found.sort_by_key(|f| std::cmp::Reverse(f.0));
    Json(json!(found
        .iter()
        .take(200)
        .map(|(_, d)| json!({"folder": d, "name": d.file_name().map(|n| n.to_string_lossy())}))
        .collect::<Vec<_>>()))
}

/// The folders the projects are in, and the home folder, each once.
fn roots(repos: &[PathBuf]) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = repos
        .iter()
        .filter_map(|r| r.parent().map(Path::to_path_buf))
        .chain([home_dir()])
        .collect();
    roots.sort();
    roots.dedup();
    roots
}

/// A folder with a `.git` folder, with when that was last changed.
fn used(d: PathBuf) -> Option<(SystemTime, PathBuf)> {
    Some((
        std::fs::metadata(d.join(".git"))
            .ok()
            .filter(|m| m.is_dir())?
            .modified()
            .ok()?,
        d,
    ))
}
