//! The work there is and what of it reached main: the autopilot projects with and without startable items, and the lanes merged in 24 h.
use super::compute::Ctx;
use super::facts::{empty_plan_note, paused_of};
use super::metric::{ids, secs, Metric};
use crate::app::{Lane, Shared};
use crate::plan;
/// The autopilot projects: those with startable items (and how many), and those with none.
pub(super) struct Work {
    pub(super) open: Vec<(String, usize)>,
    pub(super) empty: Vec<String>,
}
/// Open items of a project the autopilot works on: only items it can start (no step-0 role items, no owner-wait items).
fn startable(app: &Shared, p: &crate::config::Project) -> usize {
    plan::read(app, &p.id)
        .map(|s| {
            s.iter()
                .flat_map(|s| &s.items)
                .filter(|i| !i.done && !crate::lanes::is_autopilot_skipped(&i.text))
                .count()
        })
        .unwrap_or(0)
}
pub(super) fn work(c: &Ctx) -> Work {
    let (open, empty): (Vec<(String, usize)>, Vec<_>) = c
        .cfg
        .projects
        .iter()
        .filter(|p| crate::lanes::autopilot_on(c.app, &p.id))
        .map(|p| (p.id.clone(), startable(c.app, p)))
        .partition(|(_, n)| *n > 0);
    let empty: Vec<String> = empty.into_iter().map(|(p, _)| p).collect();
    Work { open, empty }
}
/// When a lane reached main (its end, for a lane merged before that was kept).
pub(super) fn merged_end(l: &Lane) -> Option<i64> {
    l.merged_at
        .as_deref()
        .or(l.ended_at.as_deref())
        .and_then(secs)
}
/// The lanes merged in the last 24 h.
pub(super) fn merged<'a>(c: &'a Ctx) -> Vec<&'a Lane> {
    c.lanes
        .iter()
        .filter(|l| l.status == "merged" && merged_end(l).is_some_and(|m| m >= c.day))
        .collect()
}
pub(super) fn merged_metric(c: &Ctx, w: &Work, merged: &[&Lane]) -> Metric {
    let open_txt = w
        .open
        .iter()
        .map(|(p, n)| format!("{p} {n}"))
        .collect::<Vec<_>>()
        .join(", ");
    let why = if w.empty.is_empty() {
        "no empty plans: lanes that do not reach main are the problem"
    } else {
        "no startable work in the plan (not a lane failure) for:"
    };
    let paused: String = w.open.iter().map(|(p, _)| paused_of(c.app, p)).collect();
    let facts = format!(
        "merged in 24 h: [{}]; open items in autopilot projects: [{open_txt}]; {why}{}{paused}",
        ids(merged),
        empty_plan_note(&w.empty)
    );
    let live = !w.open.is_empty() && !super::pass::younger(c.app, c.day); // ten a day asks nothing of a first day
    c.mk("merged", merged.len() as f64, live, facts)
}
