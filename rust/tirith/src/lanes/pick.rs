//! Starting the next lane: the oldest queued one, or else the autopilot's next item, in a project not blocked now.
use super::autopilot::next_item;
use super::blocked::blocked;
use super::cleanup::cleanup_later;
use super::launch::launch;
use crate::app::{now_iso, Lane, Shared};
use crate::config::{Config, Project};
use crate::manager;
use std::collections::HashMap;

/// Whether the project may start a lane now; a new reason it may not is said once, and kept as `paused:<project>`.
fn may_start(
    app: &Shared,
    p: &Project,
    lanes: &[Lane],
    said: &mut HashMap<String, String>,
) -> bool {
    let why = blocked(app, p, lanes);
    why.as_ref()
        .filter(|w| said.get(&p.id) != Some(*w))
        .into_iter()
        .for_each(|w| _ = app.event("", &p.id, "paused", "", &format!("no new lane: {w}"), ""));
    app.set_kv(&format!("paused:{}", p.id), why.as_deref());
    said.insert(p.id.clone(), why.clone().unwrap_or_default());
    why.is_none()
}
/// A lane that could not start fails, and the manager is told.
fn not_started(app: &Shared, next: &Lane, e: String) {
    app.update_lane(&next.id, |x| {
        (x.status, x.note, x.ended_at) = ("failed".into(), e.clone(), Some(now_iso()))
    })
    .into_iter()
    .for_each(|l| cleanup_later(app, l, false, 0));
    manager::emit_event(
        app,
        &next.project,
        "lane.not_started",
        &format!("lane {} could not start: {e}", next.id),
        None,
    );
}
pub(super) async fn start_next(
    app: &Shared,
    cfg: &Config,
    lanes: &[Lane],
    rr: &mut usize,
    said: &mut HashMap<String, String>,
) {
    let mut ok = |p: &Project| may_start(app, p, lanes, said);
    let mut queued: Vec<&Lane> = lanes.iter().filter(|l| l.status == "queued").collect();
    queued.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    let pick = queued
        .into_iter()
        .find(|l| cfg.project(&l.project).is_none_or(&mut ok))
        .cloned();
    let Some(next) = pick.or_else(|| next_item(app, lanes, rr, &mut ok)) else {
        return;
    };
    match launch(app, next.clone()).await {
        Ok(()) => app.sched_wake.notify_one(),
        Err(e) => not_started(app, &next, e),
    }
}
