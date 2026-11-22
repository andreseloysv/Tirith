//! A pair that proves nothing: both its runs failed, or in a replay a side asked for what its capture lacks or never reached the hook that answers it.
use super::store::Experiment;
use crate::app::Shared;
use rusqlite::params;

/// The replay pair (A's run `a`, B's run `b`) proves nothing (`why`): kept as a comparison with the verdict `void`, which no tally counts as a tie, a win or a loss.
pub(super) fn void(app: &Shared, e: &Experiment, (a, b): (i64, i64), why: &str) {
    let sql = "INSERT INTO lab_comparisons (experiment, a_turn, b_turn, b_is, verdict, why) VALUES (?1, ?2, ?3, '', 'void', ?4)";
    app.insert(sql, params![e.id, a, b, why]);
}

/// Why replayed run `id` proves nothing, given the calls it had no answer for and the ones the hook heard; none when it does.
pub(super) fn why(app: &Shared, id: i64, (missed, heard): (usize, usize)) -> Option<String> {
    if missed > 0 {
        return Some(format!(
            "{missed} call(s) asked for something the capture lacks"
        ));
    }
    let sql = "SELECT COALESCE(steps, 0) FROM turns WHERE id = ?1";
    let steps = app.query(sql, [id], |r| r.get::<_, i64>(0));
    let unheard = heard == 0 && steps.first().is_some_and(|n| *n > 0);
    unheard.then(|| "its tool calls never reached Tirith's hook".to_string())
}
