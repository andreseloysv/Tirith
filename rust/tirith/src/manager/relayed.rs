//! An agent's answer of a card (`tirith answer`): the manager closes one the owner answered in the chat. An approval, or a payment, is never the agent's to give: it takes the owner's own words (`owner_words.rs`), written in that conversation after the card was asked.
use crate::app::Shared;
use serde_json::{json, Value};

const REFUSED: &str = "an approval or a payment is the owner's: they answer the card on the page or their phone, or write in this conversation. Never answer it yourself: end your turn, their answer starts your next one";

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
        "approval" | "payment" => {
            super::owner_words::after(app, app.conv(&project), &at).ok_or(REFUSED)?
        }
        _ => answer.to_string(),
    };
    super::answer::owner_answer(app, id, &answer, true)
}
