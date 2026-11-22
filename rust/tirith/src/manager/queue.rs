//! Reports and events for a conversation that wait for its next turn.
use super::deliver::deliver;
use super::talk::talk;
use crate::app::Shared;
use std::time::Duration;

/// The reports kept for one conversation; older ones are counted and said, not forgotten silently.
const KEPT: usize = 30;

/// A routed event for one conversation: with `turn` handed over (gathered with the other events of the next `limits.batch_seconds`, so many events cost one turn) and its `phone` marking that turn's answer; with `note` it waits for the conversation's next message. Anything else hears nothing.
pub(super) fn queue(app: &Shared, project: &str, text: &str, mode: &str, phone: bool) {
    let turn = match mode {
        "turn" => true,
        "note" => false,
        _ => return,
    };
    let first = talk(project, |t| {
        t.notes.push(text.to_string());
        let cut = t.notes.len().saturating_sub(KEPT);
        t.left_out += cut;
        t.notes.drain(..cut);
        t.phones |= turn && phone;
        turn && !std::mem::replace(&mut t.due, true)
    });
    if first {
        let (app, project, batch) = (
            app.clone(),
            project.to_string(),
            app.cfg().factory.limits.batch_seconds,
        );
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(batch)).await;
            talk(&project, |t| t.due = false);
            deliver(&app, &project, "", false);
        });
    }
}
