//! The shared cargo build folders: one per project, removed for projects with nothing under way when the disk runs low.
use crate::app::{Lane, Shared};
use std::path::{Path, PathBuf};

/// One cargo build folder per project, shared by its lanes and checks: a folder per worktree costs gigabytes each.
pub fn shared_target(app: &Shared, project: &str) -> PathBuf {
    app.home.join("build").join(project).join("cargo")
}
/// Below the factory's `limits.low_disk_gb` free, before the autopilot pauses: the shared build caches (`<home>/build/<project>`) of projects with no lane or stage under way are removed; the next build makes them again.
pub(super) fn clear_idle_builds(root: &Path, busy: &[String]) -> Vec<PathBuf> {
    std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|d| d.is_dir() && !busy.iter().any(|b| d.file_name() == Some(b.as_ref())))
        .filter(|d| std::fs::remove_dir_all(d).is_ok())
        .collect()
}
/// The projects with a lane or a stage run under way.
fn busy_projects(app: &Shared, lanes: &[Lane]) -> Vec<String> {
    let mut busy: Vec<String> = lanes
        .iter()
        .filter(|l| {
            l.pid.is_some() || matches!(l.status.as_str(), "running" | "waiting" | "merging")
        })
        .map(|l| l.project.clone())
        .collect();
    busy.extend(app.query(
        "SELECT project FROM runs WHERE status = 'running'",
        [],
        |r| r.get::<_, String>(0),
    ));
    busy
}
pub(super) async fn free_idle_builds(app: &Shared, lanes: &[Lane]) {
    let busy = busy_projects(app, lanes);
    let root = app.home.join("build");
    let gone = tokio::task::spawn_blocking(move || clear_idle_builds(&root, &busy))
        .await
        .unwrap_or_default();
    gone.iter().for_each(|d| {
        _ = app.event(
            "",
            &d.file_name().unwrap_or_default().to_string_lossy(),
            "disk",
            "diet",
            &format!(
                "removed {} (less than 5 GB free; no lane or stage of its project under way)",
                d.display()
            ),
            "",
        )
    });
}
