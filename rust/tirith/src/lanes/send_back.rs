//! Sending a lane's work back to its agent with the reason, twice at most.
use super::cleanup::cleanup_later;
use crate::app::{Lane, Shared};
use crate::manager;

/// The third time: the lane fails, and the manager is told.
fn give_up(app: &Shared, lane: &Lane, reason: &str) {
    app.update_lane(&lane.id, |l| {
        (l.status, l.note) = ("failed".into(), reason.chars().take(2000).collect())
    })
    .into_iter()
    .for_each(|l| cleanup_later(app, l, false, 0));
    manager::emit_event(
        app,
        &lane.project,
        "lane.failed",
        &format!(
            "lane {} ({}: {}) failed after {} tries: {}",
            lane.id,
            lane.project,
            lane.item,
            lane.attempts + 1,
            reason.lines().next().unwrap_or("")
        ),
        None,
    );
}
/// The merge failed because of the lane: the same agent continues in its slot with the reason, twice at most.
pub fn send_back(app: &Shared, lane: &Lane, reason: &str) {
    if lane.attempts >= 2 {
        return give_up(app, lane, reason);
    }
    app.update_lane(&lane.id, |l| {
        (l.attempts, l.solo, l.resume, l.note, l.status) =
            (l.attempts + 1, false, format!("OpenBot sent your work back:\n\n{reason}\n\nFix it on your branch, commit, and end your turn."), format!("sent back: {}", reason.lines().next().unwrap_or("")), "queued".into())
    });
    app.sched_wake.notify_one();
}
