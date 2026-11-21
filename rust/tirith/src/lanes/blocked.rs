//! Why a project cannot start a lane now, and what fixes it.
use super::disk::free_gb;
use crate::app::{Lane, Shared};
use crate::{config::Project, guard};

/// Why a project cannot start a lane now (merge backlog, disk, commit guard), or `None`.
pub(super) fn blocked(app: &Shared, p: &Project, lanes: &[Lane]) -> Option<String> {
    let (lim, backlog) = (
        &app.cfg().factory.limits,
        lanes
            .iter()
            .filter(|l| {
                l.project == p.id && matches!(l.status.as_str(), "merge-queued" | "merging")
            })
            .count(),
    );
    (backlog >= lim.merge_backlog)
        .then(|| format!("{backlog} lanes wait for the merge queue"))
        .or_else(|| {
            (free_gb(&app.home.join("lanes")) < lim.low_disk_gb).then(|| {
                format!(
                    "less than {} GB free on the disk of the lanes' worktrees",
                    lim.low_disk_gb
                )
            })
        })
        .or_else(|| guard::intact(app, p).err())
}
pub fn paused_fix(project: &str, why: &str) -> String {
    let said = why
        .find('(')
        .zip(why.rfind(')'))
        .map(|(a, b)| &why[a + 1..b])
        .filter(|f| f.starts_with("tirith "))
        .map(String::from);
    let known = [
        (
            "merge queue",
            "wait for the merges (tirith status shows them)",
        ),
        ("GB free", "free disk space"),
    ]
    .into_iter()
    .find(|(w, _)| why.contains(w))
    .map(|(_, f)| f.to_string());
    said.or(known).unwrap_or_else(|| {
        if why.contains("core.hooksPath") || why.contains("not Tirith's") {
            "fix the git hooks by hand, then tirith guard install ".to_string() + project
        } else {
            format!("tirith guard install {project}")
        }
    })
}
