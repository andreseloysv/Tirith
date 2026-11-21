//! After main moved: the deploy is owed, main is green, the push goes out, the lanes are merged and gone, and everyone hears it.
use super::train::{Riders, Train};
use super::{deploy::deploy, news::tell_news, pipeline::close_pipeline, red::clear_red};
use crate::app::{git, now_iso, Shared};
use crate::config::Project;
use crate::lanes;
pub(super) async fn landed(app: &Shared, t: &Train, riders: Riders, sha: &str, changed: &[String]) {
    let project = &t.project;
    // Main has moved: its deploy is owed from here on, so a daemon stopped before the end of this function deploys when it is back.
    let owed = format!("deploy_due:{}", project.id);
    let _ = app
        .kv(&owed)
        .is_none()
        .then(|| app.set_kv(&owed, Some("0@0")));
    close_pipeline(app, &t.pipe, "passed");
    clear_red(app, &project.id);
    let sha8 = &sha[..8.min(sha.len())];
    push(app, project).await;
    mark_merged(app, &riders, sha8).await;
    tell_news(app, project, &riders.done(), sha8, changed);
    app.sched_wake.notify_one();
    deploy(app, project, changed).await;
}
/// Main goes to origin when pushing is on and the repo has one; a failed push is an error event.
async fn push(app: &Shared, project: &Project) {
    let repo = project.repo();
    if app.cfg().push && git(&repo, &["remote", "get-url", "origin"]).await.is_ok() {
        drop(
            git(&repo, &["push", "-q", "origin", "HEAD"])
                .await
                .map_err(|e| app.event("", &project.id, "error", "push", &e, "")),
        );
    }
}
/// Each lane is merged (a tick says it was already true on main) and cleaned up.
async fn mark_merged(app: &Shared, riders: &Riders, sha8: &str) {
    for l in &riders.done() {
        let tick_only = riders.ticks.iter().any(|t| t.id == l.id);
        app.update_lane(&l.id, |x| {
            (x.status, x.merged_at, x.note) = (
                "merged".into(),
                Some(now_iso()),
                if tick_only {
                    "already true on main".into()
                } else {
                    format!("on main at {sha8}")
                },
            )
        });
        lanes::cleanup_lane(app, l, true).await; // merged: its worktree, branch, logs and brief go now (its ignored files move to the project first)
    }
}
