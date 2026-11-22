//! A replay pair that proves nothing: a side asked for something its capture lacks.
use super::store::Experiment;
use crate::app::Shared;
use rusqlite::params;

/// The replay pair (A's run `a`, B's run `b`) proves nothing (`why`): kept as a comparison with the verdict `void`, which no tally counts as a tie, a win or a loss.
pub(super) fn void(app: &Shared, e: &Experiment, (a, b): (i64, i64), why: &str) {
    let sql = "INSERT INTO lab_comparisons (experiment, a_turn, b_turn, b_is, verdict, why) VALUES (?1, ?2, ?3, '', 'void', ?4)";
    app.insert(sql, params![e.id, a, b, why]);
}
