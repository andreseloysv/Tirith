//! Stopping a lane: its agent ends, the lane is stopped and cleaned up.
use super::cleanup::cleanup_later;
use crate::app::{kill_group, now_iso, Lane, Shared};
use std::time::Duration;

/// SIGTERM to the agent's group, SIGKILL five seconds later.
pub(super) fn kill_agent(pid: Option<u32>) {
    let Some(pid) = pid else { return };
    kill_group(pid, crate::app::TERM);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        kill_group(pid, crate::app::KILL);
    });
}
pub fn stop(app: &Shared, id: &str) -> Result<Lane, String> {
    let lane = app.lane(id).ok_or(format!("no lane '{id}'"))?;
    matches!(
        lane.status.as_str(),
        "queued" | "running" | "waiting" | "merge-queued"
    )
    .then_some(())
    .ok_or_else(|| format!("lane {id} is {}", lane.status))?;
    kill_agent(lane.pid);
    app.update_lane(id, |l| {
        (l.status, l.ended_at, l.pid) = ("stopped".into(), Some(now_iso()), None)
    })
    .inspect(|l| {
        cleanup_later(
            app,
            l.clone(),
            false,
            if lane.pid.is_some() { 6 } else { 0 },
        )
    })
    .ok_or("lane vanished".into())
}
