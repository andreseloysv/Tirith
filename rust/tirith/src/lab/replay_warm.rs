//! A replay's uncounted first turn: what every turn of that agent shares (its tools, its instructions) is paid and cached here, not by whichever side happens to run first.
use super::store::Experiment;
use crate::app::Shared;
use crate::turns::Captured;

/// The variant on its row: neither A nor B, so no number counts it.
pub(super) const WARM: &str = "W";
/// Its task: none of the captures', so neither side finds its own already cached.
const TASK: &str = "Use no tool. Answer with the one word: ready";

pub(super) async fn warm(app: &Shared, e: &Experiment, turn: i64) {
    let cap = Captured {
        turn,
        start: TASK.into(),
        calls: vec![],
    };
    let _ = super::replay_side::side(app, e, &cap, WARM).await;
}
