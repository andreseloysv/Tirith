//! The exec session: one exec turn of Codex, Copilot or Muse per hand-over, resumed under its kept session id.
use super::deliver::report;
use super::exec_stream::on_event;
use super::prompt::{history, prompt};
use super::talk::{set_busy, talk};
use super::typing::{begin_turn, typing_start, typing_stop};
use crate::app::Shared;
use crate::codex::Exec;
use crate::rooms;
pub(super) async fn exec_session(app: &Shared, project: &str, kind: Exec) -> Result<(), String> {
    let key = format!("manager_session:{}:{project}", kind.alias());
    let resumed = app.kv(&key);
    let _ = resumed.is_none().then(|| crate::turns::fresh(app, project));
    // The turn's message in plain text, with the recent conversation in front of it when the agent starts fresh; `sent` is handed over again when the turn never started.
    let sent = talk(project, |t| std::mem::take(&mut t.pending));
    begin_turn(app, project); // this exec turn received exactly that waiting input; what arrives while it runs waits for the next one
    let mut input = if resumed.is_none() {
        history(app, project)
    } else {
        String::new()
    };
    input += &sent.iter().map(|s| format!("{s}\n")).collect::<String>();
    let mut started = false;
    let result = crate::codex::turn(
        app,
        &app.chat_role(project),
        project,
        resumed.as_deref(),
        (&prompt(app, project), &input),
        |msg| {
            crate::turns::line(project, msg);
            on_event(app, project, &key, kind, msg)
        },
        |pid| {
            started = true;
            crate::turns::begin(app, project);
            exec_started(app, project, pid);
        },
    )
    .await;
    if !started {
        talk(project, |t| {
            (
                t.reply_to.append(&mut t.turn_rooms),
                drop(t.pending.splice(0..0, sent)),
            )
        }); // no turn ran: its rooms wait again with its input
        return result.map(|_| ());
    }
    exec_end(app, project, &key, kind, result);
    Ok(())
}
/// The exec turn's process runs: the conversation is busy and the phones see it typing.
fn exec_started(app: &Shared, project: &str, pid: u32) {
    talk(project, |t| {
        (
            (t.pid, t.stale) = (pid, false),
            set_busy(app, project, t, true),
        )
    });
    typing_start(app, project, false);
}
/// The exec turn ended: its answer (or why it failed) goes to the chat, the phones and its rooms.
fn exec_end(app: &Shared, project: &str, key: &str, kind: Exec, result: Result<String, String>) {
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
}
