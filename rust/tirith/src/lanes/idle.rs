//! An idle project's build cache and merge folder go once it has been long enough without work.
use super::stamp::at_s;
use super::worktree::remove_worktree;
use crate::app::{Lane, Shared};
use crate::config::Project;
use std::path::Path;

/// What the sweep knows once for every project: now, the idle limit, and which projects a manager or a pipeline keeps busy.
pub(super) struct Idle {
    pub now: u64,
    pub minutes: u64,
    pub talking: Vec<String>,
    pub pipelines: Vec<String>,
}
/// An idle project's build cache and merge folder go together once it has been `minutes` without work; busy, or not that long yet, keeps both.
pub(super) async fn drop_idle_cache(
    repo: &Path,
    build: &Path,
    merge_dir: &Path,
    busy: bool,
    last: u64,
    now: u64,
    minutes: u64,
) -> bool {
    let Some(()) = (!busy && now.saturating_sub(last) >= minutes * 60).then_some(()) else {
        return false;
    };
    (
        remove_worktree(repo, merge_dir).await,
        (!build.exists() || std::fs::remove_dir_all(build).is_ok()) && !merge_dir.exists(),
    )
        .1
}
/// The newest end of the project's stage runs.
fn last_run(app: &Shared, project: &Project) -> u64 {
    app.query(
        "SELECT MAX(COALESCE(ended_at, started_at)) FROM runs WHERE project = ?1",
        [&project.id],
        |r| r.get::<_, String>(0),
    )
    .first()
    .map(|s| at_s(Some(s)))
    .unwrap_or(0)
}
/// When the folder last changed, in seconds since the epoch.
fn modified_s(dir: &Path) -> u64 {
    std::fs::metadata(dir)
        .and_then(|m| m.modified())
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        })
        .unwrap_or(0)
}
/// The last time a project was seen working: the newest of its lanes' and stage runs' stamps, and its build cache's own age.
fn last_activity(app: &Shared, project: &Project, lanes: &[Lane], build: &Path) -> u64 {
    lanes
        .iter()
        .filter(|l| l.project == project.id)
        .map(|l| {
            at_s(l.ended_at.as_deref())
                .max(at_s(l.started_at.as_deref()))
                .max(l.last_activity)
        })
        .max()
        .unwrap_or(0)
        .max(last_run(app, project))
        .max(modified_s(build))
}
/// Whether a manager, a pipeline or a live lane of the project works now.
fn busy(idle: &Idle, p: &Project, lanes: &[Lane]) -> bool {
    idle.talking.contains(&p.id)
        || idle.pipelines.contains(&p.id)
        || lanes.iter().any(|l| {
            l.project == p.id
                && (l.pid.is_some() || matches!(l.status.as_str(), "running" | "merging"))
        })
}
/// The idle diet of one project, said as an event when it removed something.
pub(super) async fn diet_project(app: &Shared, p: &Project, lanes: &[Lane], idle: &Idle) {
    let (repo, build, merge_dir) = (
        p.repo(),
        app.home.join("build").join(&p.id),
        p.lanes_dir().join("merge"),
    );
    let Some(()) = (build.exists() || merge_dir.exists()).then_some(()) else {
        return;
    };
    let (minutes, last) = (idle.minutes, last_activity(app, p, lanes, &build));
    let _ = drop_idle_cache(&repo, &build, &merge_dir, busy(idle, p, lanes), last, idle.now, minutes)
        .await
        .then(|| app.event("", &p.id, "disk", "diet", &format!("removed {} and its merge folder ({minutes} min with no lane, stage or manager working)", build.display()), ""));
}
