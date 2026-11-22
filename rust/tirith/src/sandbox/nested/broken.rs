//! When a limited Codex is stopped while it runs: what would run its commands is gone, or was never its.
use super::{bridge::Bridge, stalled::stalled};
use std::time::{Duration, Instant};

/// Codex connects at its start, before it reads a prompt or asks its model.
const CONNECTS_WITHIN: Duration = Duration::from_secs(20);

/// Why it is stopped now; none while it may go on.
pub(super) fn broken(bridge: &Bridge, runs: u32, began: Instant) -> Option<&'static str> {
    if bridge.lost() || stalled(runs) {
        return Some("the process that runs its commands ended or was stopped, so Codex was stopped: none of them runs in its own process");
    }
    (!bridge.linked() && began.elapsed() > CONNECTS_WITHIN).then_some(
        "this Codex did not hand its commands to the process that has their limit, so it was stopped",
    )
}
