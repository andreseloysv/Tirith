//! A lane that ended for good leaves nothing behind: its work saved, its ignored files kept, its worktree and (merged) branch and logs gone.
use super::keep::keep_ignored;
use super::worktree::remove_worktree;
use crate::app::{alive, git, Lane, Shared};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A finished lane's paper: its log, the logs of its checks and stages, and its brief.
pub(super) fn drop_lane_logs(app: &Shared, id: &str) {
    let _ = (
        std::fs::remove_file(app.home.join(format!("logs/lanes/{id}.log"))),
        std::fs::remove_file(app.home.join(format!("briefs/{id}.md"))),
    );
    app.query("SELECT log FROM runs WHERE COALESCE(log, '') <> '' AND ',' || lane || ',' LIKE '%,' || ?1 || ',%'", [id], |r| r.get::<_, String>(0)).iter().for_each(|log| drop(std::fs::remove_file(log)));
}
/// Whether another active lane holds the same worktree.
fn held_by_other(app: &Shared, lane: &Lane) -> bool {
    app.lanes
        .lock()
        .unwrap()
        .values()
        .any(|l| l.active() && l.id != lane.id && l.workspace == lane.workspace)
}
/// Commits whatever is left uncommitted in the worktree on the lane's branch.
async fn save_uncommitted(app: &Shared, lane: &Lane, ws: &Path) {
    let dirty = !git(ws, &["status", "--porcelain"])
        .await
        .unwrap_or_default()
        .is_empty();
    if !dirty {
        return;
    }
    drop(git(ws, &["add", "-A"]).await);
    drop(
        git(ws, &["commit", "-qm", "Saved by NeoBot"])
            .await
            .map_err(|e| {
                let why = format!("could not save the work left in {}: {e}", ws.display());
                app.event(&lane.id, &lane.project, "error", "cleanup", &why, "")
            }),
    );
}
/// A lane that merged, stopped or failed for good leaves nothing behind: first its uncommitted work is committed on its branch (a merged lane's work is on main already, and its branch goes below), the git-ignored files of its worktree that are no build cache move to the same path in the project (kept ignored there, never overwriting), then the worktree goes with its caches (`git worktree remove` and prune) and with the lane's temporary folder (`temp.rs`). A merged lane's branch, logs and brief go at once; a stopped or failed one keeps its branch, and its logs go after 24 h (the sweep).
pub async fn cleanup_lane(app: &Shared, lane: &Lane, merged: bool) {
    let Some(project) = app
        .cfg()
        .project(&lane.project)
        .cloned()
        .filter(|_| !lane.pid.is_some_and(alive))
    else {
        return;
    };
    let (repo, ws) = (project.repo(), PathBuf::from(&lane.workspace));
    if !lane.workspace.is_empty() && ws.is_dir() && !held_by_other(app, lane) {
        if !merged {
            save_uncommitted(app, lane, &ws).await;
        }
        keep_ignored(&ws, &repo, &lane.id).await;
        remove_worktree(&repo, &ws).await;
        super::temp::remove(&ws);
    }
    if merged && !lane.branch.is_empty() {
        drop(git(&repo, &["branch", "-D", &lane.branch]).await);
    }
    drop((
        merged.then(|| drop_lane_logs(app, &lane.id)),
        app.update_lane(&lane.id, |l| {
            l.workspace = if l.workspace == lane.workspace {
                String::new()
            } else {
                l.workspace.clone()
            }
        }),
    ));
}
/// The cleanup of a lane that just ended for good, off the caller's path: after `wait_s` (a stopped lane's agent is gone first).
pub fn cleanup_later(app: &Shared, lane: Lane, merged: bool, wait_s: u64) {
    let app = app.clone();
    tokio::spawn(async move {
        (
            tokio::time::sleep(Duration::from_secs(wait_s)).await,
            cleanup_lane(&app, &lane, merged).await,
        )
            .1
    });
}
