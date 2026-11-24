//! The loose turn: one fresh turn of a `session = "turn"` role, its text the answer.
use super::deliver::user_line;
use super::spawn::spawn_chat;
use super::turn_read::read_turn;
use crate::app::{kill_group, Shared};
use crate::models;
use serde_json::Value;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
/// One fresh turn of a `session = "turn"` role: its factory prompt as its instructions, `input` as its one message, its `access` deciding whether it may write; the assistant's text until the turn ends is the answer. Any turn role runs this way (the CEO included).
pub async fn run_turn(
    app: &Shared,
    role: &str,
    project: &str,
    input: &str,
) -> Result<String, String> {
    let prompt = super::prompt::instructions(app, role);
    let mut rec = crate::turns::turn_of(app, role, project);
    let Some(()) = crate::codex::Exec::of(&models::role(app, role).agent)
        .is_none()
        .then_some(())
    else {
        let each = |m: &Value| rec.line(m);
        let result = crate::codex::turn(app, role, project, None, (&prompt, input), each, |_| {});
        let result = result.await;
        return crate::turns::done(app, rec, result);
    };
    let read_only = app.cfg().factory.role::<String>(role, "access").as_deref() == Some("read");
    let (mut child, mut stdin, stdout) = match open(app, (role, project), prompt, input).await {
        Ok(open) => open,
        Err(e) => return crate::turns::done(app, rec, Err(e)), // it never started: its row says why
    };
    let pid = child.id().unwrap_or(0);
    let mut lines = BufReader::new(stdout).lines();
    let mut text = String::new();
    let read = read_turn(&mut lines, &mut stdin, read_only, (&mut text, &mut rec));
    let finished = tokio::time::timeout(Duration::from_secs(600), read).await;
    kill_group(pid, crate::app::KILL);
    let _ = child.wait().await;
    let text = text.trim().to_string();
    let result = finished
        .map_err(|_| {
            format!(
                "no answer within 10 minutes{}",
                if text.is_empty() {
                    String::new()
                } else {
                    format!(": {text}")
                }
            )
        })
        .and_then(|_| {
            if rec.usage.failed {
                return Err(format!("its agent failed: {text}")); // its text is the error
            }
            (!text.is_empty())
                .then_some(text)
                .ok_or_else(|| "the session ended without an answer".to_string())
        });
    crate::turns::done(app, rec, result)
}
/// The turn's process, handed its one message.
async fn open(
    app: &Shared,
    (role, project): (&str, &str),
    prompt: String,
    input: &str,
) -> Result<(Child, ChildStdin, ChildStdout), String> {
    let mut cmd = models::chat_argv(app, role, None)?;
    cmd.argv.extend(crate::lab::replay_args(app, &cmd.alias));
    cmd.argv.extend(["--append-system-prompt".into(), prompt]);
    let (child, mut stdin, stdout) = spawn_chat(app, role, project, cmd.argv)?;
    let wrote = stdin.write_all(&send(user_line(input))).await;
    wrote.map_err(|e| e.to_string())?;
    Ok((child, stdin, stdout))
}
pub(super) fn send(line: String) -> Vec<u8> {
    (line + "\n").into_bytes()
}
