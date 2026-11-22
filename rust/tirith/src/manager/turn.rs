//! The loose turn: one fresh turn of a `session = "turn"` role, its text the answer.
use super::deliver::user_line;
use super::spawn::{control_reply, spawn_chat, turn_permission};
use crate::app::{kill_group, Shared};
use crate::models;
use serde_json::Value;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{ChildStdin, ChildStdout};
/// One fresh turn of a `session = "turn"` role: its factory prompt as its instructions, `input` as its one message, its `access` deciding whether it may write; the assistant's text until the turn ends is the answer. Any turn role runs this way (the CEO included).
pub async fn run_turn(
    app: &Shared,
    role: &str,
    project: &str,
    input: &str,
) -> Result<String, String> {
    let prompt = app.cfg().factory.prompts_of(role);
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
    let mut cmd = models::chat_argv(app, role, None)?;
    cmd.argv.extend(["--append-system-prompt".into(), prompt]);
    let (mut child, mut stdin, stdout) = spawn_chat(app, role, project, cmd.argv)?;
    let pid = child.id().unwrap_or(0);
    let mut lines = BufReader::new(stdout).lines();
    stdin
        .write_all(&send(user_line(input)))
        .await
        .map_err(|e| e.to_string())?;
    let mut text = String::new();
    let read = read_turn(&mut lines, &mut stdin, read_only, (&mut text, &mut rec));
    let finished = tokio::time::timeout(Duration::from_secs(600), read).await;
    kill_group(pid, libc::SIGKILL);
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
            (!text.is_empty())
                .then_some(text)
                .ok_or_else(|| "the session ended without an answer".to_string())
        });
    crate::turns::done(app, rec, result)
}
fn send(line: String) -> Vec<u8> {
    (line + "\n").into_bytes()
}
/// Reads the turn until its `result` (true) or the end of its output (false), gathering its text and answering its permission requests.
async fn read_turn(
    lines: &mut Lines<BufReader<ChildStdout>>,
    stdin: &mut ChildStdin,
    read_only: bool,
    (text, rec): (&mut String, &mut crate::turns::Turn),
) -> bool {
    while let Ok(Some(line)) = lines.next_line().await {
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        rec.line(&msg);
        match msg["type"].as_str().unwrap_or("") {
            "assistant" => msg["message"]["content"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|b| b["type"] == "text")
                .for_each(|b| *text += &format!("{}\n", b["text"].as_str().unwrap_or(""))),
            "result" => return true,
            "control_request" if msg["request"]["subtype"] == "can_use_tool" => {
                let behavior = turn_permission(
                    read_only,
                    msg["request"]["tool_name"].as_str().unwrap_or(""),
                    &msg["request"]["input"],
                );
                let _ = stdin.write_all(&send(control_reply(&msg, behavior))).await;
            }
            _ => {}
        }
    }
    false
}
