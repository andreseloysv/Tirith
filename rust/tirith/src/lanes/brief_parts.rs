//! The parts of a lane's brief taken from the plan, the other lanes and the project's rules.
use crate::app::{Lane, Shared};
use crate::plan;

/// The project's lanes now.
pub(super) fn project_lanes(app: &Shared, project: &str) -> Vec<Lane> {
    app.lanes
        .lock()
        .unwrap()
        .values()
        .filter(|l| l.project == project)
        .cloned()
        .collect()
}
/// The items of the lane's step as checklist lines, each open one with the newest lane on it.
pub(super) fn step_items(step: Option<&plan::Step>, lane: &Lane, lanes: &[Lane]) -> Vec<String> {
    let tag = |i: &plan::Item| {
        lanes
            .iter()
            .filter(|l| l.step == lane.step && plan::same(&l.item, &i.text))
            .max_by(|a, b| a.created_at.cmp(&b.created_at))
            .filter(|_| !i.done)
            .map(|l| format!("  ({}: {})", l.id, l.status))
            .unwrap_or_default()
    };
    step.map(|s| &s.items[..])
        .unwrap_or_default()
        .iter()
        .map(|i| {
            format!(
                "- [{}] {}{}",
                if i.done { "x" } else { " " },
                i.text,
                tag(i)
            )
        })
        .collect()
}
/// The other lanes of the project that work or wait now.
pub(super) fn others(lane: &Lane, lanes: &[Lane]) -> String {
    let others: Vec<String> = lanes
        .iter()
        .filter(|l| l.id != lane.id && matches!(l.status.as_str(), "running" | "waiting"))
        .map(|l| format!("- {} ({}): {}", l.id, l.status, l.item))
        .collect();
    if others.is_empty() {
        "none".into()
    } else {
        others.join("\n")
    }
}
/// The project's rules from its manifest, one line each.
pub(super) fn rules(app: &Shared, lane: &Lane) -> String {
    let rules: Vec<String> = app
        .cfg()
        .project(&lane.project)
        .map(|p| app.manifest(&p.repo()).rules)
        .unwrap_or_default()
        .iter()
        .map(|r| format!("- {}", r.message))
        .collect();
    rules.join("\n")
}
