//! An agent's answer of a card (`tirith answer`): the manager closes one the owner answered in the chat. An approval is never the agent's to give: it takes the owner's own words, written in that conversation after the card was asked.
use crate::app::Shared;
use serde_json::{json, Value};

const REFUSED: &str = "an approval is the owner's: they answer the card on the page or their phone, or write in this conversation. Never answer it yourself: end your turn, their answer starts your next one";

/// The answer an agent hands in for card (or waiting lane) `id`.
pub fn relayed(app: &Shared, id: &str, answer: &str) -> Result<Value, String> {
    if app.lane(id).is_some_and(|l| l.status == "waiting") {
        return crate::lanes::answer(app, id, answer, "the manager").map(|_| json!({"ok": true}));
    }
    let sql = "SELECT kind, project, at FROM owner_items WHERE id = ?1 AND status = 'open'";
    let cards = app.query(sql, [id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    });
    let (kind, project, at) = cards
        .into_iter()
        .next()
        .ok_or(format!("no open question '{id}'"))?;
    let answer = match kind.as_str() {
        "approval" => said(app, app.conv(&project), &at).ok_or(REFUSED)?,
        _ => answer.to_string(),
    };
    super::answer::owner_answer(app, id, &answer, true)
}

/// The owner's last words in `conv` after `since`, from the page or a paired phone.
fn said(app: &Shared, conv: &str, since: &str) -> Option<String> {
    let sql = "SELECT role, text FROM chat WHERE project = ?1 AND at > ?2 AND role IN ('owner', 'room') ORDER BY rowid DESC";
    let lines = app.query(sql, [conv, since], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    });
    lines.into_iter().find_map(|(role, text)| {
        let words = if role == "owner" {
            text.as_str()
        } else {
            // `<author> (<room>): <text>`: a phone's author ends with what only pairing gives
            let (head, words) = text.split_once("): ")?;
            crate::rooms::paired_phone(head.rsplit_once(" (")?.0).then_some(words)?
        };
        (!words.trim().is_empty()).then(|| words.trim().chars().take(600).collect())
    })
}
