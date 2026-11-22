//! The exec session: one exec turn of Codex, Copilot or Muse per hand-over, resumed under its kept session id.
use super::exec_end::{exec_end, exec_started};
use super::exec_stream::on_event;
use super::jobs::take_group;
use super::prompt::{opening, prompt};
use super::talk::talk;
use super::typing::begin_turn;
use crate::app::Shared;
use crate::codex::Exec;
pub(super) async fn exec_session(app: &Shared, project: &str, kind: Exec) -> Result<(), String> {
    let key = format!("manager_session:{}:{project}", kind.alias());
    let resumed = app.kv(&key);
    let _ = resumed.is_none().then(|| crate::turns::fresh(app, project));
    // The turn's message in plain text, with the recent conversation in front of it when the agent starts fresh; `sent` is handed over again when the turn never started.
    let sent = talk(project, |t| {
        let sent = take_group(&mut t.pending);
        super::waiting::starts(t, &sent);
        sent
    });
    begin_turn(app, project); // this exec turn received exactly that waiting input; what arrives while it runs waits for the next one
    let mut input = if resumed.is_none() {
        opening(app, project)
    } else {
        String::new()
    };
    input += &sent
        .iter()
        .map(|m| format!("{}\n", m.text))
        .collect::<String>();
    let by = sent.first().map(|m| m.by.clone()).unwrap_or_default();
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
            crate::turns::handed(app, project, by.clone());
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
