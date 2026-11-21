//! A lane whose agent ended out of credits, or without commits: the next agent of the chain, or failed.
use super::cleanup::cleanup_later;
use crate::app::{git, now_iso, Lane, Shared};
use crate::{manager, models};

/// The lane failed: the manager is told, and its cleanup follows.
fn failed(app: &Shared, lane: &Lane, why: &str) {
    let text = format!(
        "lane {} ({}: {}) failed: {why}",
        lane.id, lane.project, lane.item
    );
    manager::emit_event(app, &lane.project, "lane.failed", &text, None);
    app.lane(&lane.id)
        .into_iter()
        .for_each(|l| cleanup_later(app, l, false, 0));
}
/// Out of credits: the next agent of the lane role's fallback chain continues the work, or the lane fails when none is left.
pub(super) fn out_of_credits(app: &Shared, lane: &Lane) {
    let next = models::fallback(app, &models::lane_role(&app.cfg()), &lane.tried);
    app.update_lane(&lane.id, |l| {
        l.pid = None;
        l.ended_at = Some(now_iso());
        if let Some((alias, model)) = &next {
            l.agent = alias.clone();
            l.model = model.clone();
            l.session.clear();
            l.status = "queued".into();
            l.note = format!("out of credits: continues with {alias}");
            l.resume = "The previous agent ran out of credits. Continue the work already committed on your branch.".into();
        } else {
            l.status = "failed".into();
            l.note = format!("out of credits, no agent left to try (tried {})", l.tried.join(", "));
        }
    });
    if next.is_none() {
        failed(app, lane, "out of credits on every agent of the lane role");
    }
}
/// Ended without commits: nothing to merge.
pub(super) fn without_commits(app: &Shared, lane: &Lane, code: Option<i32>) {
    app.update_lane(&lane.id, |l| {
        (l.pid, l.ended_at, l.status, l.note) = (
            None,
            Some(now_iso()),
            "failed".into(),
            format!(
                "ended without commits (exit {}): nothing to merge",
                code.map(|c| c.to_string()).unwrap_or("?".into())
            ),
        )
    });
    failed(app, lane, "it ended without commits, nothing to merge");
}
/// True when the lane's branch has no commit beyond the project's HEAD (it is known and countable). Without a check on its item such a lane failed; with one it may still be done (the check can already be true): it goes to the merge queue, where the check runs on main.
pub(super) async fn no_commits(app: &Shared, lane: &Lane) -> bool {
    let Some(p) = app
        .cfg()
        .projects
        .iter()
        .find(|p| p.id == lane.project)
        .cloned()
    else {
        return false;
    };
    !lane.branch.is_empty()
        && matches!(
            git(
                &p.repo(),
                &["rev-list", "--count", &format!("HEAD..{}", lane.branch)]
            )
            .await
            .as_deref(),
            Ok("0")
        )
}
