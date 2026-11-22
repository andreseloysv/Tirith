//! Handing a conversation its messages: to its live session, or waiting for the session that starts for them.
use super::compose::compose;
use super::jobs::{reserve, start_run};
use super::run::run;
use super::talk::{set_busy, talk};
use super::typing::{begin_turn, typing_start};
use crate::app::Shared;
use crate::turns::By;
use crate::{bots, rooms};
use serde_json::json;
pub(super) fn report(app: &Shared, project: &str, text: &str) {
    app.chat("tirith", text, project);
    rooms::to_phones(app, app.conv(project), &format!("Tirith: {text}"));
}
/// One user message as a stream-json line. `pending` keeps plain texts; only a live session's stdin sees lines.
pub(super) fn user_line(text: &str) -> String {
    json!({"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": text}]}}).to_string()
}
/// Hands the conversation its waiting reports and `text` as one message; its session starts when none runs. With nothing to say it hands the next routine's run, when the conversation is free.
pub fn deliver(app: &Shared, project: &str, text: &str, phones: bool) {
    hand(app, project, text, phones, None);
}
/// `deliver`, the turn it starts begun by `by` (the owner, a room), not by what waited.
pub(super) fn hand(app: &Shared, project: &str, text: &str, phones: bool, by: Option<By>) {
    let memory = bots::context(app, project); // a bot's memory, read at every turn
    let job = text.is_empty().then(|| reserve(project)).flatten();
    job.iter().for_each(|j| start_run(app, project, j));
    let says = !text.is_empty() || job.is_some() || talk(project, |t| !t.notes.is_empty());
    let _ = says.then(|| crate::turns::prepare(app, project)); // (git runs here, not under the conversations' lock)
    let (start, handed, began) = talk(project, |t| {
        let was_busy = t.busy;
        t.phones |= phones || job.as_ref().is_some_and(|j| j.phones);
        let Some(msg) = compose(t, project, (memory, text), (job.as_ref(), by)) else {
            return (false, false, false);
        };
        let line = user_line(&msg.text);
        let live = t.tx.as_ref();
        let sent = !t.stale && live.is_some_and(|tx| tx.send(line).is_ok());
        if sent {
            crate::turns::handed(app, project, msg.by);
            set_busy(app, project, t, true);
        } else {
            t.pending.push(msg); // the session on its way takes it, and only then is the manager busy
        }
        (
            !std::mem::replace(&mut t.alive, true),
            true,
            sent && !was_busy,
        )
    });
    let _ = began.then(|| begin_turn(app, project)); // a message a live, idle session took starts its turn now; one for a running turn begins at that turn's end
    let _ = handed.then(|| typing_start(app, project, true)); // the phones see the turn start (or, for a turn already running, its state again), even while its session is still on its way
    let _ = start.then(|| tokio::spawn(run(app.clone(), project.to_string())));
}
