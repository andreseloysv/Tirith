//! What a landed merge tells: the lanes still working, the manager, and the page.
use crate::app::{Lane, Shared};
use crate::config::Project;
use crate::{lanes, manager, plan};
use serde_json::json;
pub(super) fn tell_news(
    app: &Shared,
    project: &Project,
    done: &[Lane],
    sha8: &str,
    changed: &[String],
) {
    let items: Vec<&str> = done.iter().map(|l| l.item.as_str()).collect();
    tell_working(app, &project.id, &items, changed);
    // A merge is news, not a question: the manager gets a turn for it only when it leaves the plan without open work.
    manager::emit_event(
        app,
        &project.id,
        "merge.landed",
        &merged_note(&project.id, sha8, &items),
        None,
    );
    let _ = plan::read(app, &project.id)
        .is_ok_and(|steps| !steps.iter().flat_map(|s| &s.items).any(|i| !i.done && !lanes::is_autopilot_skipped(&i.text)))
        .then(|| manager::emit_event(app, &project.id, "plan.empty", "The plan has no open item left: tell the owner what is done, and ask for the next goal if the project needs one.", None));
    app.emit("plan", json!({"project": project.id}));
}
/// Every lane of the project with a running agent hears what main got and which files changed.
fn tell_working(app: &Shared, project: &str, items: &[&str], changed: &[String]) {
    let working: Vec<String> = app
        .lanes
        .lock()
        .unwrap()
        .values()
        .filter(|l| l.project == project && l.pid.is_some())
        .map(|l| l.id.clone())
        .collect();
    let files = changed
        .iter()
        .take(10)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    working.iter().for_each(|id| {
        lanes::tell(
            id,
            &format!(
                "[tirith] main advanced: {} (files: {files})",
                items.join("; ")
            ),
        )
    });
}
/// The `merged on …` note: only the beginning of each item — up to its first ": ", " (" or ". ", 120 chars at most — never the whole item (one reached 1,826 chars).
pub(super) fn merged_note(project: &str, sha8: &str, items: &[&str]) -> String {
    format!(
        "merged on {project} at {sha8}: {}",
        items.iter().map(|i| head(i)).collect::<Vec<_>>().join("; ")
    )
}
/// An item up to its first ": ", " (" or ". ", 120 chars at most.
fn head(i: &str) -> String {
    i[..[": ", " (", ". "]
        .iter()
        .filter_map(|d| i.find(d))
        .min()
        .unwrap_or(i.len())]
        .chars()
        .take(120)
        .collect()
}
