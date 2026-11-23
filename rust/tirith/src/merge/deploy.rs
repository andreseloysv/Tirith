//! Runs the project's deploy stages on main as it is now, in the merge slot, so what is on main is what is deployed. After a merge only the stages whose `paths` match what it changed run; with no files named, or while an earlier deploy has not passed yet, all of them do. A failure never undoes or holds back a merge: the manager is told once, and the worker tries again after ten minutes, then after twice as long each time, six hours at most. `deploy_due:<project>` is "<when to try again>@<the wait since the last failure, 0 for none>"; "0@.." while a deploy is owed or runs, so one that a restart cut short is due at once.
use super::pipeline::{close_pipeline, open_pipeline, Pipe};
use super::{paths::deploys, paths::wanted, slot::merge_slot, slot::reset, stage::run_stage};
use crate::app::{git, now_s, Shared};
use crate::config::{Manifest, Project, Stage};
use crate::manager;
pub(super) async fn deploy(app: &Shared, project: &Project, changed: &[String]) {
    let (manifest, key) = (
        app.manifest(&project.repo()),
        format!("deploy_due:{}", project.id),
    );
    let prev = app
        .kv(&key)
        .and_then(|v| v.split_once('@')?.1.parse::<u64>().ok())
        .unwrap_or(0);
    let changed = if prev > 0 { &[][..] } else { changed };
    let stages: Vec<&Stage> = manifest
        .stages
        .iter()
        .filter(|s| deploys(s) && (changed.is_empty() || wanted(s, changed)))
        .collect();
    let Some(()) = (!stages.is_empty()).then_some(()) else {
        return app.set_kv(&key, None);
    }; // nothing to deploy for this merge, or no deploy stage at all: nothing is owed
    app.set_kv(&key, Some(&format!("0@{prev}")));
    let sha = git(&project.repo(), &["rev-parse", "HEAD"])
        .await
        .unwrap_or_default();
    let pipe = open_pipeline(app, &project.id, "deploy", "", &sha);
    let failed = run_deploy(app, project, &manifest, &pipe, &sha, stages)
        .await
        .err();
    close_pipeline(
        app,
        &pipe,
        if failed.is_none() { "passed" } else { "failed" },
    );
    settle(app, project, &key, prev, &sha, failed);
}
async fn run_deploy(
    app: &Shared,
    project: &Project,
    manifest: &Manifest,
    pipe: &Pipe,
    sha: &str,
    stages: Vec<&Stage>,
) -> Result<(), String> {
    let slot = merge_slot(project, manifest).await?;
    reset(&slot, sha).await?;
    for st in stages {
        // Each try starts with the usual time limit: a deploy that hangs must not hold the merge slot twice as long every time.
        app.set_kv(&format!("stage_learned:{}:{}", project.id, st.name), None);
        run_stage(app, pipe, &slot, &st.name, &st.run).await?;
    }
    Ok(())
}
/// A failed deploy is due again after a doubling wait; the manager hears the first failure and each success.
fn settle(
    app: &Shared,
    project: &Project,
    key: &str,
    prev: u64,
    sha: &str,
    failed: Option<String>,
) {
    let (wait, sha8, pid) = (
        if prev == 0 {
            600
        } else {
            (prev * 2).min(6 * 3600)
        },
        &sha[..8.min(sha.len())],
        &project.id,
    );
    app.set_kv(
        key,
        failed
            .as_ref()
            .map(|_| format!("{}@{wait}", now_s() + wait))
            .as_deref(),
    );
    match failed {
        None => manager::emit_event(app, pid, "deploy.done", &format!("deployed {pid} at {sha8}"), None),
        Some(e) if prev == 0 => {
            manager::emit_event(app, pid, "deploy.failed", &format!("the deploy of {pid} at {sha8} failed. The merge stays on main; OpenBot tries the deploy again by itself until it passes (Retry on its pipeline: at once).\n{e}"), None)
        }
        _ => {}
    }
}
