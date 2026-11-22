//! The loose turn of a turn role (`run_turn`): started by what waits under `turn:<role>`, kept with its result.
use super::live::take_trigger;
use super::record::Turn;
use super::save::finish;
use crate::app::Shared;

/// One fresh turn of a turn role (`run_turn`), started by what waits under `turn:<role>` (an event, when nobody said).
pub fn turn_of(app: &Shared, role: &str, project: &str) -> Turn {
    let (trigger, routine) = take_trigger(&format!("turn:{role}"));
    let trigger = if trigger.is_empty() {
        "event".into()
    } else {
        trigger
    };
    let conv = if project.is_empty() { role } else { project };
    let mut turn = Turn::new(app, conv, role, (trigger, routine));
    turn.loose = true; // its whole session
    super::interrupted::mark(app, &turn);
    turn
}
/// A turn role's turn ended with `result`: its row is kept, and the result goes back as it was.
pub fn done(app: &Shared, t: Turn, result: Result<String, String>) -> Result<String, String> {
    let answer = result.clone().unwrap_or_else(|e| e);
    finish(app, t, Some(result.is_ok()), Some(&answer));
    result
}
