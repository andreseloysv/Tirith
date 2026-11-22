//! Which lane and project a hook call comes from.
use crate::app::{Lane, Shared};
use crate::config::Project;

/// For the refusals of a call without a token, the working lane whose worktree holds its cwd: it names nobody.
pub(super) fn lane_at(app: &Shared, cwd: &str) -> Option<Lane> {
    let in_worktree = |l: &&Lane| {
        matches!(l.status.as_str(), "running" | "waiting")
            && !l.workspace.is_empty()
            && std::path::Path::new(cwd).starts_with(&l.workspace)
    };
    let lanes = app.lanes.lock().unwrap();
    lanes.values().find(in_worktree).cloned()
}

/// The lane's project, else the project whose repository or lanes folder holds the cwd.
pub(super) fn project_of(app: &Shared, lane: Option<&Lane>, cwd: &str) -> Option<Project> {
    let cfg = app.cfg();
    let at = |p: &&Project| {
        std::path::Path::new(cwd).starts_with(p.repo())
            || std::path::Path::new(cwd).starts_with(p.lanes_dir())
    };
    lane.and_then(|l| cfg.project(&l.project))
        .or_else(|| cfg.projects.iter().filter(|p| !p.repo.is_empty()).find(at))
        .cloned()
}
