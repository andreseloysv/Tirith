//! A bot's turn ended: a run of a live experiment keeps its experiment and variant on its row, a B run is compared with the A before it, and the experiment may be decided.
use super::seat::taken;
use super::store::running_one;
use crate::app::Shared;
use rusqlite::params;

/// Turn `id` of conversation `conv`, which ran routine `routine`, was kept: if it was an experiment's run, its row says which, and the bot is back in its own folder.
pub fn finished(app: &Shared, id: i64, conv: &str, routine: &str) {
    let Some(seat) = taken(app, conv, routine) else {
        return;
    };
    app.insert(
        "UPDATE turns SET experiment = ?1, variant = ?2 WHERE id = ?3",
        params![seat.experiment, seat.variant, id],
    );
    let Some(e) = running_one(app, seat.experiment) else {
        return;
    };
    if e.mode == "shadow" {
        super::shadow_pair::done(app, &e, (&seat.tree, seat.pair), (Some(id), None));
    // compared once its B ended too
    } else if seat.variant == "B" {
        crate::manager::end_session(conv); // its next session starts in the bot's own folder again
        super::compare::compare(app, &e, id);
    }
    super::decide::decide(app, e.id);
}
