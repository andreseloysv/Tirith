//! The owner's answer to an open card or to a waiting lane.
use super::cards::{card_done, note_line};
use super::deliver::report;
use super::events::emit_event;
use super::queue::queue;
use crate::app::Shared;
use serde_json::{json, Value};
/// The owner's own words, in the project's chat as the owner's message (a typed answer was only a note of Tirith's there); not when the manager closes a card the owner already answered in the chat.
fn said(app: &Shared, project: &str, answer: &str, by_manager: bool) {
    if !by_manager {
        app.chat("owner", answer, project);
    }
}
/// The owner's answer to an open card: straight to the lane whose question it carries (the manager is told), else to the manager of the card's project with the question it answers. `by_manager`: the manager closes a card the owner answered in the chat.
pub fn owner_answer(
    app: &Shared,
    id: &str,
    answer: &str,
    by_manager: bool,
) -> Result<Value, String> {
    if let Some(l) = app.lane(id).filter(|l| l.status == "waiting") {
        crate::lanes::answer(app, id, answer, "the owner")?;
        said(app, &l.project, answer, by_manager);
        note_line(
            app,
            &l.project,
            &format!(
                "The owner answered lane {id} ({}) directly: {answer}",
                l.question
            ),
        );
        return Ok(json!({"ok": true}));
    }
    let sql =
        "SELECT kind, project, question, lane FROM owner_items WHERE id = ?1 AND status = 'open'";
    let (kind, project, question, lane): (String, String, String, String) = app
        .db
        .lock()
        .unwrap()
        .query_row(sql, [id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .map_err(|_| format!("no open question '{id}'"))?;
    said(app, &project, answer, by_manager);
    card_done(app, id, "answered", answer);
    let text = format!("The owner answered {id} ({kind}: {question}): {answer}");
    emit_event(app, &project, "card.answered", &text, None);
    if !lane.is_empty() && crate::lanes::answer(app, &lane, answer, "the owner").is_ok() {
        note_line(
            app,
            &project,
            &format!("{text}\nIt went straight to lane {lane}."),
        );
    } else {
        report(app, &project, &text);
        if !by_manager {
            crate::turns::on_event(app.conv(&project), "card.answered", "turn");
            queue(app, app.conv(&project), &text, "turn", true);
        }
    }
    Ok(json!({"ok": true}))
}
