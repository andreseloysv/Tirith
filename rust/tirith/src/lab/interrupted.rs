//! A replay the daemon was restarted in the middle of: its runs were in memory, so it never finishes. At start it is decided if all its pairs ran and were judged, and ended honestly otherwise, never decided later on the pairs it happened to have.
use super::decide::{close, decide};
use super::store::all_running;
use crate::app::Shared;

/// At the daemon's start: every replay still running is decided (its pairs were all run and judged) or discarded, saying why.
pub fn interrupted(app: Shared) {
    for e in all_running(&app).into_iter().filter(|e| e.mode == "replay") {
        let finished = app.kv(&super::replay::done_key(e.id)).is_some();
        if finished && super::numbers::unjudged(&app, e.id) == 0 {
            decide(&app, e.id);
        } else {
            let done = super::numbers::tally(&app, e.id);
            let n = done.wins + done.ties + done.losses + done.voids;
            let line = format!("Descarté {} {}: el demonio se reinició durante la repetición y solo había {n} pares comparados", e.routine, e.branch);
            close(&app, &e, false, &line);
        }
    }
}
