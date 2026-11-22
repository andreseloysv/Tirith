//! The stream-json session of a conversation: started with what waits for it, read until it ends or rests.
use super::deliver::user_line;
use super::exec::exec_session;
use super::prompt::{history, prompt};
use super::session_end::{close, not_ready, Live};
use super::spawn::spawn_chat;
use super::stream::{read, Stream};
use super::talk::{set_busy, talk, Msg};
use super::typing::{begin_turn, typing_start};
use crate::app::Shared;
use crate::models;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;
use tokio::sync::mpsc;
/// One session of a conversation, until it ends or rests. Err when it never got ready: its messages wait again.
pub(super) async fn session(app: &Shared, project: &str) -> Result<(), String> {
    let role = app.chat_role(project); // a bot may have a model of its own, under its key
    let alias = models::role(app, &role).agent;
    let Some(()) = (!matches!(alias.as_str(), "codex" | "muse")).then_some(()) else {
        return exec_session(app, project, alias == "muse").await;
    };
    let key = format!("manager_session:{alias}:{project}");
    let resumed = app.kv(&key);
    let history = resumed.is_none().then(|| history(app, project));
    let mut cmd = models::chat_argv(app, &role, resumed.as_deref())?;
    cmd.argv
        .extend(["--append-system-prompt".into(), prompt(app, project)]);
    let (child, stdin, stdout) = spawn_chat(app, "manager", project, cmd.argv)?;
    let pid = child.id().unwrap_or(0);
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    let sent = hand_pending(app, project, pid, &tx, history.as_deref());
    let writer = tokio::spawn(write_lines(stdin, rx));
    let stream = Stream {
        app,
        project,
        key: &key,
        tx: &tx,
    };
    let (ready, rest) = read(&stream, idle_of(app, &role), stdout).await;
    close(app, project, Live { child, pid, writer }, ready).await;
    crate::turns::abandon(app, project); // a turn the session left unanswered failed
    let Some(()) = (!ready && !rest).then_some(()) else {
        return Ok(());
    };
    not_ready(app, project, &key, sent, resumed.is_some())
}
/// The new session takes the conversation's waiting messages (the first with the recent chat in front when it starts fresh), each beginning a turn of its own in order.
fn hand_pending(
    app: &Shared,
    project: &str,
    pid: u32,
    tx: &mpsc::UnboundedSender<String>,
    history: Option<&str>,
) -> Vec<Msg> {
    let sent: Vec<Msg> = talk(project, |t| {
        (t.pid, t.tx, t.stale) = (pid, Some(tx.clone()), false);
        set_busy(app, project, t, true);
        let sent: Vec<Msg> = std::mem::take(&mut t.pending);
        sent.iter().enumerate().for_each(|(i, m)| {
            let first = if i == 0 { history.unwrap_or("") } else { "" };
            drop(tx.send(user_line(&format!("{first}{}", m.text))));
            crate::turns::handed(app, project, m.by.clone());
        });
        sent
    });
    let _ =
        (!sent.is_empty()).then(|| (begin_turn(app, project), typing_start(app, project, false))); // the waiting turn starts now; a session restarted after a failure shows it again
    sent
}
/// Writes every line handed to the session to its stdin, until that fails or the session ends.
async fn write_lines(mut stdin: ChildStdin, mut rx: mpsc::UnboundedReceiver<String>) {
    while let Some(line) = rx.recv().await {
        let Some(()) = stdin
            .write_all(format!("{line}\n").as_bytes())
            .await
            .is_ok()
            .then_some(())
        else {
            break;
        };
        let _ = stdin.flush().await;
    }
}
/// How long a quiet session with no turn running lives: its role's `idle_minutes` (30 by default).
fn idle_of(app: &Shared, role: &str) -> Duration {
    Duration::from_secs(
        app.cfg()
            .factory
            .role::<u64>(role, "idle_minutes")
            .unwrap_or(30)
            * 60,
    )
}
