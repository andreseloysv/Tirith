//! The pass at startup and then every few minutes that applies the cleanup rules to everything that already meets them.
use super::cleanup::{cleanup_lane, drop_lane_logs};
use super::heal::heal_excludes;
use super::idle::{diet_project, Idle};
use super::stamp::at_s;
use crate::app::{git, now_s, Lane, Shared};
use crate::manager;
use std::path::Path;

/// One ended lane: a merged one still holding a worktree, log or brief is cleaned now, a stopped or failed one still holding a worktree a minute after it ended too, and the logs and brief of one stopped or failed more than 24 h ago go.
async fn sweep_lane(app: &Shared, l: &Lane) {
    let (log, brief, ended) = (
        app.home.join(format!("logs/lanes/{}.log", l.id)),
        app.home.join(format!("briefs/{}.md", l.id)),
        at_s(l.ended_at.as_deref()),
    );
    let (merged, held_dir) = (
        l.status == "merged",
        !l.workspace.is_empty() && Path::new(&l.workspace).is_dir(),
    );
    if (merged && (held_dir || log.exists() || brief.exists()))
        || (!merged && held_dir && now_s().saturating_sub(ended) > 60)
    {
        cleanup_lane(app, l, merged).await;
    }
    let _ = (l.status != "merged"
        && (log.exists() || brief.exists())
        && ended > 0
        && now_s().saturating_sub(ended) >= 24 * 3600)
        .then(|| drop_lane_logs(app, &l.id));
}
/// What the idle diet needs to know once for all projects.
fn idle_now(app: &Shared) -> Idle {
    Idle {
        now: now_s(),
        minutes: app.cfg().factory.limits.build_idle_minutes,
        talking: manager::busy(),
        pipelines: app.query(
            "SELECT project FROM pipelines WHERE status = 'running'",
            [],
            |r| r.get::<_, String>(0),
        ),
    }
}
/// The pass at startup and then every few minutes, applying the same rules to everything that already meets them: a merged, stopped or failed lane still holding a worktree loses it now, the logs and brief of one stopped or failed more than 24 h ago go, what earlier cleanups moved into a project but git still sees as new (a `\`, space or `[` in its name broke its per-file exclude line) gets its folder excluded, and a project's build cache and merge folder go after `limits.build_idle_minutes` with no lane, stage or manager of it working.
pub(super) async fn sweep_cleanups(app: &Shared, lanes: &[Lane]) {
    for l in lanes
        .iter()
        .filter(|l| matches!(l.status.as_str(), "merged" | "stopped" | "failed"))
    {
        sweep_lane(app, l).await;
    }
    let idle = idle_now(app);
    for p in &app.cfg().projects {
        let (repo, Ok(gitdir)) = (
            p.repo(),
            git(&p.repo(), &["rev-parse", "--git-common-dir"]).await,
        ) else {
            continue;
        };
        heal_excludes(&repo, &gitdir).await;
        diet_project(app, p, lanes, &idle).await;
    }
}
