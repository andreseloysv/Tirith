//! A lane's worktree slot: a fresh one on its branch when it starts, gone in full when it ends.
use crate::app::{add_worktree, git, Lane, Shared};
use crate::config::Project;
use std::path::{Path, PathBuf};

/// Takes one worktree away in full (`git worktree remove`, then prune; a folder git will not remove is deleted): no folder and no registration left.
pub(super) async fn remove_worktree(repo: &Path, ws: &Path) {
    drop((
        (ws.exists()
            && git(
                repo,
                &["worktree", "remove", "--force", &ws.to_string_lossy()],
            )
            .await
            .is_err())
        .then(|| std::fs::remove_dir_all(ws)),
        git(repo, &["worktree", "prune"]).await,
    ));
}
/// The next free `slot-<n>` folder: one past the highest there, with no temporary folder of an older lane beside it.
pub(super) fn next_slot(dir: &Path) -> PathBuf {
    let mut nums: Vec<u32> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.strip_prefix("slot-")?.parse().ok())
        .collect();
    nums.sort();
    let slot = dir.join(format!("slot-{}", nums.last().map_or(1, |n| n + 1)));
    super::temp::remove(&slot);
    slot
}
/// A fresh worktree slot of the project for this lane alone, checked out on its `lane/<id>` branch: at main for a new branch, where the branch stands when a lane continues after its old worktree went. No slot is kept free or reused: a finished lane's slot is already gone (see `cleanup_lane`).
pub(super) async fn take_slot(
    app: &Shared,
    project: &Project,
    lane: &Lane,
) -> Result<PathBuf, String> {
    let (repo, dir) = (project.repo(), project.lanes_dir());
    let slot = next_slot(&dir);
    add_worktree(&repo, &slot, None, &app.manifest(&repo).lane_exclude)
        .await
        .inspect_err(|_| drop(std::fs::remove_dir_all(&slot)))?;
    let (known, main) = (
        git(&repo, &["rev-parse", "--verify", "-q", &lane.branch])
            .await
            .is_ok(),
        git(&repo, &["rev-parse", "HEAD"]).await?,
    );
    let args: Vec<&str> = if known {
        vec!["checkout", "-q", "-f", &lane.branch]
    } else {
        vec!["checkout", "-q", "-f", "-B", &lane.branch, &main]
    };
    git(&slot, &args).await?;
    Ok(slot)
}
