//! A card's end: its final state, the line that says so, and closing one that is no longer needed.
use super::deliver::report;
use super::events::emit_event;
use super::queue::queue;
use crate::app::Shared;
use serde_json::{json, Value};
/// A card's answer or closing, for the manager's next turn: a line in the project's chat and on the phones, and a note (no turn of its own).
pub(super) fn note_line(app: &Shared, project: &str, text: &str) {
    report(app, project, text);
    queue(app, app.conv(project), text, "note", false);
}
/// Sets a card's final state in the table and on the page.
pub(super) fn card_done(app: &Shared, id: &str, status: &str, answer: &str) {
    let _ = app.db.lock().unwrap().execute(
        "UPDATE owner_items SET status = ?1, answer = ?2 WHERE id = ?3",
        rusqlite::params![status, answer, id],
    );
    crate::turns::released(app, id); // a turn that waited for this card as its approval is done
    app.emit(
        "owner",
        json!({"id": id, "status": status, "answer": answer}),
    );
}
/// `tirith card close` (`by` the manager) or the card's Close (`by` the owner): an open card that is no longer needed is closed with its reason, said in its project's chat and on the phones, so nobody is told to ignore it.
pub fn close_card(app: &Shared, id: &str, by: &str, why: &str) -> Result<Value, String> {
    let sql = "SELECT project, kind FROM owner_items WHERE id = ?1 AND status = 'open'";
    let (project, kind): (String, String) = app
        .db
        .lock()
        .unwrap()
        .query_row(sql, [id], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|_| format!("no open card '{id}'"))?;
    let no_yes = matches!(kind.as_str(), "approval" | "payment") && by != "the owner";
    let note = format!(
        "closed by {by}{}{}",
        if no_yes {
            " (which approves nothing)"
        } else {
            ""
        },
        if why.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", why.trim())
        }
    );
    card_done(app, id, "closed", &note);
    emit_event(
        app,
        &project,
        "card.closed",
        &format!("Card {id} {note}"),
        None,
    );
    note_line(app, &project, &format!("Card {id} {note}"));
    Ok(json!({"ok": true}))
}
