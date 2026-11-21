//! What a poll decided is done: lines sent, the phones' model controls answered, cards answered, the rest handed to the manager.
use super::{outbox::queue, poll::Out, store::update, Room, PHONE};
use crate::{app::Shared, manager};
pub(super) fn act(app: &Shared, r: &Room, out: Out) {
    out.send
        .iter()
        .for_each(|(text, readers)| queue(&r.id, text, readers.clone()));
    out.controls.iter().for_each(|(peer, request)| {
        queue(
            &r.id,
            &format!(
                "tirith-models-state {}",
                crate::models::phone_response(app, peer, request)
            ),
            vec![peer.clone()],
        )
    });
    for (author, text) in &out.deliver {
        let Some(()) = (!phone_model_command(app, r, author, text)).then_some(()) else {
            continue;
        };
        let _ = (!answers_a_card(app, r, author, text))
            .then(|| manager::from_room(app, &r.project, &r.id, author, text));
    }
    out.hooks
        .iter()
        .for_each(|h| crate::bots::webhook(app, &r.project, h));
    out.mentions
        .into_iter()
        .for_each(|m| app.emit("room-mention", m));
}
/// `o12: yes` from a paired phone answers that card by code (a phone writes `O12`); everything else is for the manager.
fn answers_a_card(app: &Shared, r: &Room, author: &str, text: &str) -> bool {
    let card = text
        .split_once(':')
        .filter(|(_, answer)| author.ends_with(PHONE) && !answer.trim().is_empty());
    let carded = card.is_some_and(|(id, answer)| {
        manager::owner_answer(app, &id.trim().to_lowercase(), answer.trim(), false).is_ok()
    });
    let _ = carded.then(|| {
        update(app, &r.id, |r| {
            (
                r.owes.owed.retain(|o| o.1 != *text),
                r.owes.answering.retain(|o| o.1 != *text),
            )
        })
    });
    carded
}
/// `/model` and the like from a paired phone are run here, not handed to the manager.
pub(super) fn phone_model_command(app: &Shared, r: &Room, author: &str, text: &str) -> bool {
    let Some(()) = (author.ends_with(PHONE)
        && matches!(
            text.split_whitespace().next(),
            Some("/model" | "/models" | "/modelo" | "/modelos")
        ))
    .then_some(()) else {
        return false;
    };
    app.chat("room", &format!("{author} ({}): {text}", r.id), &r.project);
    manager::model_command(app, app.conv(&r.project), text);
    update(app, &r.id, |r| {
        r.owes
            .owed
            .retain(|o| o != &(author.to_string(), text.to_string()))
    });
    true
}
