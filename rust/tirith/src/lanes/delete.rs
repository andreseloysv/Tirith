//! `tirith lane delete`: a lane thrown away with its branch, its worktree and its open plan item.
use super::stop::kill_agent;
use super::worktree::remove_worktree;
use crate::app::{git, Lane, Shared};
use crate::plan;
use std::path::Path;
use std::time::Duration;

/// The lane leaves the map, the database and the page.
fn forget(app: &Shared, id: &str) {
    app.lanes.lock().unwrap().remove(id);
    let _ = app
        .db
        .lock()
        .unwrap()
        .execute("DELETE FROM lanes WHERE id = ?1", [id]);
    app.emit("lane", serde_json::json!({"id": id, "deleted": true}));
}
/// Its worktree goes in full, unless another lane has that folder now (this lane is out of the map already); with it gone, its branch can be deleted.
async fn drop_folder_and_branch(app: &Shared, repo: &Path, lane: &Lane) {
    let held = app
        .lanes
        .lock()
        .unwrap()
        .values()
        .any(|l| l.workspace == lane.workspace);
    if !held && !lane.workspace.is_empty() {
        remove_worktree(repo, Path::new(&lane.workspace)).await;
    }
    if !lane.branch.is_empty() {
        drop(git(repo, &["branch", "-D", &lane.branch]).await);
    }
}
/// `tirith lane delete`: the lane is thrown away. Its agent stops, its branch and unsaved work go, it leaves every list, and its open item leaves the plan, so nothing starts it again (stopping keeps all that: the autopilot only skips the item).
pub async fn delete(app: &Shared, id: &str) -> Result<(), String> {
    let lane = app.lane(id).ok_or(format!("no lane '{id}'"))?;
    (lane.status != "merging")
        .then_some(())
        .ok_or_else(|| format!("lane {id} is being merged right now: try again in a minute"))?;
    kill_agent(lane.pid);
    forget(app, id);
    if lane.status != "merged" {
        let _ = plan::drop_item(app, &lane.project, &lane.step, &lane.item).await;
    }
    let Some(p) = app.cfg().project(&lane.project).cloned() else {
        return Ok(());
    };
    if lane.pid.is_some() {
        tokio::time::sleep(Duration::from_secs(6)).await; // the agent is gone (kill_agent) before its folder is touched
    }
    drop_folder_and_branch(app, &p.repo(), &lane).await;
    Ok(())
}
