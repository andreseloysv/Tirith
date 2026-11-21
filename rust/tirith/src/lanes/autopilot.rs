//! The autopilot: in round-robin over the projects, the next open plan item that no lane works on becomes a lane.
use super::skip::{is_autopilot_skipped, waits_for_open_item};
use super::start::start;
use crate::app::{Lane, Shared};
use crate::{config::Project, plan};

pub fn autopilot_on(app: &Shared, project: &str) -> bool {
    app.kv(&format!("autopilot:{project}"))
        .map(|v| v == "on")
        .unwrap_or(app.cfg().autopilot)
}
/// The autopilot is on for the project and its main is not red.
fn project_on(app: &Shared, p: &Project) -> bool {
    autopilot_on(app, &p.id) && app.kv(&format!("main_red:{}", p.id)).is_none()
}
/// The newest lane of the project on that item.
fn latest<'a>(lanes: &'a [Lane], project: &str, step: &str, item: &str) -> Option<&'a Lane> {
    lanes
        .iter()
        .filter(|l| l.project == project && l.step == step && plan::same(&l.item, item))
        .max_by(|a, b| a.created_at.cmp(&b.created_at))
}
fn skipped(l: Option<&Lane>) -> bool {
    l.is_some_and(|l| matches!(l.status.as_str(), "failed" | "stopped"))
}
/// The project's current step (the first with an open item that did not fail or stop) and its first item no lane works on, as (step, item).
fn pick_item(app: &Shared, p: &Project, lanes: &[Lane]) -> Option<(String, String)> {
    let steps = plan::read(app, &p.id).ok()?;
    let open: Vec<&str> = steps
        .iter()
        .flat_map(|s| s.items.iter())
        .filter(|i| !i.done)
        .map(|i| i.text.as_str())
        .collect();
    let newest = |step: &str, item: &str| latest(lanes, &p.id, step, item);
    let startable = |i: &plan::Item| {
        !i.done && !is_autopilot_skipped(&i.text) && !waits_for_open_item(&i.text, &open)
    };
    let st = steps.iter().find(|s| {
        s.items
            .iter()
            .any(|i| startable(i) && !skipped(newest(&s.num, &i.text)))
    })?;
    let it = st.items.iter().find(|i| {
        startable(i) && newest(&st.num, &i.text).is_none_or(|l| !l.active() && !skipped(Some(l)))
    })?;
    Some((st.num.clone(), it.text.clone()))
}
/// The autopilot's next item, started as a lane: `ok` says whether a project may start one now.
pub(super) fn next_item(
    app: &Shared,
    lanes: &[Lane],
    rr: &mut usize,
    ok: &mut dyn FnMut(&Project) -> bool,
) -> Option<Lane> {
    let cfg = app.cfg();
    let n = cfg.projects.len();
    for k in 0..n {
        let p = &cfg.projects[(*rr + k) % n];
        let Some((step, item)) = project_on(app, p)
            .then(|| pick_item(app, p, lanes))
            .flatten()
        else {
            continue;
        };
        let Some(()) = ok(p).then_some(()) else {
            continue;
        };
        *rr = (*rr + k + 1) % n;
        if let Ok(l) = start(app, &p.id, &step, &item, None)
            .map_err(|e| app.event("", &p.id, "error", "autopilot", &e, ""))
        {
            return Some(l);
        }
    }
    None
}
