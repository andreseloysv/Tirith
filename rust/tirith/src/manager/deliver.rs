//! Handing a conversation its messages: to its live session, or waiting for the session that starts for them.
use super::compose::compose;
use super::jobs::{reserve, start_run};
use super::run::run;
use super::talk::{set_busy, talk};
use super::typing::{begin_turn, typing_start};
use super::waiting::{held, starts};
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
    let job = text.is_empty().then(|| reserve(project)).flatten();
    job.iter().for_each(|j| start_run(app, project, j)); // its seat first: an experiment's B reads the branch's
    let says = !text.is_empty() || job.is_some() || talk(project, |t| t.notes_due);
    let memory = bots::handed(app, project, job.is_some(), says); // a bot's memory, read at every turn
    let _ = says.then(|| crate::turns::prepare(app, project)); // (git runs here, not under the conversations' lock)
    let (start, handed, began) = talk(project, |t| {
        let was_busy = t.busy;
        let msg = compose(t, project, (memory, text), (job.as_ref(), by));
        t.taken &= job.is_none(); // from here its message holds the conversation
        let Some(mut msg) = msg else {
            return (false, false, false);
        };
        msg.phones |= phones;
        // Into a live session, unless a routine's run has it: then only that run itself.
        let may = msg.job || !held(t) && t.pending.is_empty();
        let live = t.tx.as_ref().filter(|_| may && !t.stale);
        let sent = live.is_some_and(|tx| tx.send(user_line(&msg.text)).is_ok());
        if sent {
            starts(t, std::slice::from_ref(&msg));
            crate::turns::handed(app, project, msg.by);
            set_busy(app, project, t, true);
        } else if msg.job {
            t.pending.insert(0, msg); // the run was taken first
        } else {
            t.phones |= msg.phones && may && !was_busy; // the turn on its way is this message's
            t.pending.push(msg); // the session on its way (or the run's end) takes it, and only then is the manager busy
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
