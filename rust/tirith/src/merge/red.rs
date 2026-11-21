//! A red main: `main_red:<project>` is "<stage> at <sha8>"; the queue tests it again when main moves and clears it when green.
use super::{locks::BUSY, main_check::main_check, slot::merge_slot};
use crate::app::{git, Shared};
use crate::config::Project;
use crate::manager;
use serde_json::json;
/// A `main_red` value is "<stage> at <sha8>": it is stale when main's head is another commit.
pub(super) fn red_stage_if_stale(why: &str, head: &str) -> Option<String> {
    (head.len() >= 8 && !why.ends_with(&head[..8])).then_some(())?;
    why.rsplit_once(" at ").map(|(stage, _)| stage.to_string())
}
/// After main moves while it is red (a merge, a manual commit), the queue tests the latest main again, so the health «Main en rojo» names the current commit and clears when it passes.
pub(super) async fn recheck_red(app: &Shared) {
    let cfg = app.cfg();
    for project in &cfg.projects {
        let Some(why) = app.kv(&format!("main_red:{}", project.id)) else {
            continue;
        };
        let head = git(&project.repo(), &["rev-parse", "HEAD"])
            .await
            .unwrap_or_default();
        let Some(stage) = red_stage_if_stale(&why, &head) else {
            continue;
        };
        let _busy = BUSY.lock().await;
        recheck_stage(app, project, &stage, &head).await;
    }
}
/// Runs the red stage on the new head; still red, the value names the new head.
async fn recheck_stage(app: &Shared, project: &Project, stage: &str, head: &str) {
    let manifest = app.manifest(&project.repo());
    let (Some(st), Ok(slot)) = (
        manifest.stages.iter().find(|s| s.name == stage),
        merge_slot(project, &manifest).await,
    ) else {
        return;
    };
    if !main_check(app, project, &slot, head, st, false).await {
        app.set_kv(
            &format!("main_red:{}", project.id),
            Some(&format!("{stage} at {}", &head[..8])),
        );
        app.emit("plan", json!({"project": project.id}));
    }
}
pub(super) fn clear_red(app: &Shared, project: &str) {
    let key = format!("main_red:{project}");
    if app.kv(&key).is_some() {
        app.set_kv(&key, None);
        app.emit("plan", json!({"project": project}));
        manager::emit_event(
            app,
            project,
            "main.green",
            &format!("main of {project} is green again: the autopilot continues"),
            None,
        );
        app.sched_wake.notify_one();
        app.merge_wake.notify_one();
    }
}
