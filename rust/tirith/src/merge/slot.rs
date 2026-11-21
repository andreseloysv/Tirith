//! The project's merge slot: one worktree where trains, main checks and deploys run.
use crate::app::{add_worktree, git, Lane, Shared};
use crate::config::{Manifest, Project};
use std::path::{Path, PathBuf};
pub(super) async fn merge_slot(project: &Project, manifest: &Manifest) -> Result<PathBuf, String> {
    let slot = project.lanes_dir().join("merge");
    if !slot.exists() {
        add_worktree(&project.repo(), &slot, None, &manifest.lane_exclude)
            .await
            .map_err(|e| (drop_half_made(&slot, &e), e).1)?;
    }
    Ok(slot)
}
pub(super) fn disk_full(e: &str) -> bool {
    e.to_lowercase().contains("no space left on device")
}
/// A merge slot that failed to be made for lack of disk is deleted, so nothing half-made stays to eat space.
pub(super) fn drop_half_made(slot: &Path, e: &str) -> bool {
    disk_full(e) && std::fs::remove_dir_all(slot).is_ok()
}
pub(super) async fn reset(slot: &Path, sha: &str) -> Result<(), String> {
    git(slot, &["checkout", "-q", "-f", "--detach", sha]).await?;
    git(slot, &["clean", "-fdq"]).await.map(|_| ())
}
pub(super) fn requeue(app: &Shared, lanes: &[Lane], solo: bool, note: &str) {
    lanes.iter().for_each(|l| {
        drop(app.update_lane(&l.id, |x| {
            (x.status, x.solo, x.note) = ("merge-queued".into(), solo, note.into())
        }))
    });
}
