//! The two ends of an exec turn: its process runs, and its answer goes to the chat, the phones and its rooms.
use super::deliver::{deliver, report};
use super::talk::{set_busy, talk};
use super::typing::{typing_start, typing_stop};
use crate::app::Shared;
use crate::codex::Exec;
use crate::rooms;
/// The exec turn's process runs: the conversation is busy and the phones see it typing.
pub(super) fn exec_started(app: &Shared, project: &str, pid: u32) {
    talk(project, |t| {
        (
            (t.pid, t.stale) = (pid, false),
            set_busy(app, project, t, true),
        )
    });
    typing_start(app, project, false);
}
/// The exec turn ended: its answer (or why it failed) goes to the chat, the phones and its rooms.
pub(super) fn exec_end(
    app: &Shared,
    project: &str,
    key: &str,
    kind: Exec,
    result: Result<String, String>,
) {
    let (rooms_now, phones) = talk(project, |t| {
        (
            set_busy(app, project, t, false),
            (
                std::mem::take(&mut t.turn_rooms),
                std::mem::take(&mut t.phones),
            ),
        )
            .1
    });
    let (text, answered) = match result {
        Ok(text) => (text, true),
        Err(e) => {
            let who = kind.failed();
            let text = format!("{who}: {e}. Send your message again to retry.");
            report(app, project, &text);
            app.set_kv(key, None);
            (text, false)
        }
    };
    crate::turns::end(app, project, Some(answered), Some(&text));
    // Codex and Copilot speak while they work; Muse's exec turn answers once, at its end.
    let _ = (!kind.speaks() && answered)
        .then(|| app.chat("manager", &text, project))
        .map(|_| phones.then(|| rooms::to_phones(app, project, &text)));
    typing_stop(project); // finished, failed or timed out: the phones stop showing it typing
    rooms::reply(app, &Vec::from_iter(rooms_now), &text);
    deliver(app, project, "", false); // the next routine's run waiting, if the conversation is free
}
