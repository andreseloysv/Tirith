//! The manager: one long-lived agent session (stream-json in and out). The owner's messages go straight to
//! its stdin; its text, its tool calls and the end of each turn go straight to the page.

use crate::app::{kill_group, now_s, Shared};
use crate::{models, rooms, rules, sandbox};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

const PROMPT: &str = "You are the manager inside Tirith. The owner chats with you here: answer briefly, in the \
owner's language. You do not write project code yourself: you turn requests into plan items and start lanes \
(an agent in its own git worktree; Tirith merges its branch after the project's checks pass). Commands, run with \
your shell tool:\n  tirith status\n  tirith plan show <project>\n  tirith plan add <project> <step> \"<item text>\"\n  \
tirith lane start <project> <step> \"<item text>\"\n  tirith lane stop <lane>\n  tirith lane log <lane>\n\
Messages that start with [tirith] are reports from Tirith about lanes: act on them (start the next item, send a \
failed lane back with a clearer item, or tell the owner). Ask the owner only for what only the owner can do: \
credentials, payments, personal accounts, physical devices or real decisions.";

fn user_line(text: &str) -> String {
    json!({"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": text}]}}).to_string()
}

/// Delivers a message to the session now, or when the next session starts.
fn deliver(app: &Shared, text: &str) {
    let line = user_line(text);
    let tx = app.manager_in.lock().unwrap().clone();
    match tx {
        Some(tx) if tx.send(line.clone()).is_ok() => app.emit("busy", json!(true)),
        _ => app.manager_pending.lock().unwrap().push(line),
    }
}

/// Rooms whose messages started the current turn (their answer goes back there), the turn's text so far, and
/// the session's process group (to restart it).
static REPLY_TO: Mutex<Vec<String>> = Mutex::new(Vec::new());
static TURN_TEXT: Mutex<String> = Mutex::new(String::new());
static PID: AtomicU32 = AtomicU32::new(0);

/// A message from a room (a paired phone or another agent): shown in the chat, handed to the manager, and the
/// manager's answer to this turn goes back to that room.
pub fn from_room(app: &Shared, room: &str, author: &str, text: &str) {
    app.chat("room", &format!("{author} ({room}): {text}"));
    let mut to = REPLY_TO.lock().unwrap();
    if !to.iter().any(|r| r == room) {
        to.push(room.to_string());
    }
    drop(to);
    deliver(app, &format!("[room {room}] {author}: {text}"));
}

/// Ends the current session; the loop starts a new one with the current settings (resuming the conversation).
pub fn restart() {
    let pid = PID.load(Ordering::Relaxed);
    if pid != 0 {
        kill_group(pid, libc::SIGTERM);
    }
}

pub fn say(app: &Shared, text: &str) {
    app.chat("owner", text);
    deliver(app, text);
}

/// A report from Tirith (a lane ended, merged, failed): shown in the chat and handed to the manager.
pub fn notify(app: &Shared, text: &str) {
    app.chat("tirith", text);
    deliver(app, &format!("[tirith] {text}"));
}

pub async fn run(app: Shared) {
    let mut backoff = 2;
    loop {
        let started = now_s();
        if let Err(e) = session(&app).await {
            app.event("", "", "error", "manager", &e, "");
        }
        *app.manager_in.lock().unwrap() = None;
        app.emit("busy", json!(false));
        backoff = if now_s() - started > 60 { 2 } else { (backoff * 2).min(60) };
        tokio::time::sleep(Duration::from_secs(backoff)).await;
    }
}

async fn session(app: &Shared) -> Result<(), String> {
    let resumed = app.kv("manager_session");
    let mut cmd = models::chat_argv(app, "manager", resumed.as_deref())?;
    cmd.argv.extend(["--append-system-prompt".into(), PROMPT.into()]);
    let launch = sandbox::wrap(app, "manager", cmd.argv);
    let log = std::fs::OpenOptions::new().create(true).append(true).open(app.home.join("logs/manager.log")).map_err(|e| e.to_string())?;
    let mut command = launch.command();
    if !launch.env.iter().any(|(k, _)| k == "PATH") {
        command.env("PATH", app.child_path());
    }
    let mut child = command
        .current_dir(app.home.join("manager"))
        .env("TIRITH_ROLE", "manager")
        .env("TIRITH_HOME", &app.home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(log)
        .process_group(0)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("cannot start the manager ({}): {e}", launch.program))?;
    PID.store(child.id().unwrap_or(0), Ordering::Relaxed);
    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    for line in app.manager_pending.lock().unwrap().drain(..) {
        let _ = tx.send(line);
    }
    *app.manager_in.lock().unwrap() = Some(tx.clone());
    let writer = tokio::spawn(async move {
        while let Some(line) = rx.recv().await {
            if stdin.write_all(format!("{line}\n").as_bytes()).await.is_err() {
                break;
            }
            let _ = stdin.flush().await;
        }
    });
    let started = now_s();
    let mut lines = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
        match msg["type"].as_str().unwrap_or("") {
            "system" if msg["subtype"] == "init" => {
                if let Some(id) = msg["session_id"].as_str() {
                    app.set_kv("manager_session", Some(id));
                }
            }
            "assistant" => {
                for block in msg["message"]["content"].as_array().into_iter().flatten() {
                    match block["type"].as_str() {
                        Some("text") => {
                            let text = block["text"].as_str().unwrap_or("");
                            app.chat("manager", text);
                            TURN_TEXT.lock().unwrap().push_str(text);
                        }
                        Some("tool_use") => app.chat("tool", &rules::summary(block["name"].as_str().unwrap_or(""), &block["input"])),
                        _ => {}
                    }
                }
            }
            "result" => {
                app.emit("busy", json!(false));
                let rooms_now = std::mem::take(&mut *REPLY_TO.lock().unwrap());
                let text = std::mem::take(&mut *TURN_TEXT.lock().unwrap());
                if !rooms_now.is_empty() && !text.trim().is_empty() {
                    rooms::reply(app, &rooms_now, &text);
                }
            }
            "control_request" if msg["request"]["subtype"] == "can_use_tool" => {
                // The rules already ran in the PreToolUse hook; anything that reaches here is allowed.
                let reply = json!({"type": "control_response", "response": {"subtype": "success",
                    "request_id": msg["request_id"], "response": {"behavior": "allow", "updatedInput": msg["request"]["input"]}}});
                let _ = tx.send(reply.to_string());
            }
            _ => {}
        }
    }
    writer.abort();
    let _ = child.wait().await;
    if resumed.is_some() && now_s() - started < 20 {
        app.set_kv("manager_session", None); // the resume does not work: next time a fresh session
    }
    Err("the manager session ended".into())
}
