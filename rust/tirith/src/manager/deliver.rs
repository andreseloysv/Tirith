//! Handing a conversation its messages: to its live session, or waiting for the session that starts for them.
use super::run::run;
use super::talk::{set_busy, talk};
use super::typing::{begin_turn, typing_start};
use crate::app::Shared;
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
/// Hands the conversation its waiting reports and `text` as one message; its session starts when none runs.
pub fn deliver(app: &Shared, project: &str, text: &str, phones: bool) {
    let memory = bots::context(app, project); // a bot's memory, read at every turn
    let (start, handed, began) = talk(project, |t| {
        let was_busy = (t.phones |= phones, t.busy).1;
        let Some(()) = (!t.notes.is_empty() || !text.is_empty()).then_some(()) else {
            return (false, false, false);
        };
        let notes = std::mem::take(&mut t.notes)
            .into_iter()
            .map(|n| format!("[tirith] {n}"));
        let asked: Vec<String> = notes
            .chain((!text.is_empty()).then(|| text.to_string()))
            .collect();
        crate::turns::told(project, &asked.join("\n\n")); // what started the turn, for its capture
        let all: Vec<String> = memory.into_iter().chain(asked).collect();
        let text = all.join("\n\n");
        let sent = !t.stale
            && t.tx
                .as_ref()
                .is_some_and(|tx| tx.send(user_line(&text)).is_ok());
        if sent {
            crate::turns::handed(app, project);
            set_busy(app, project, t, true);
        } else {
            t.pending.push(text); // the session on its way takes it, and only then is the manager busy
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
