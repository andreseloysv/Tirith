//! The manager: one long-lived agent session (stream-json in and out). The owner's messages go straight to
//! its stdin; its text, its tool calls and the end of each turn go straight to the page.

use crate::app::{kill_group, now_iso, now_s, Shared};
use crate::{models, rooms, rules, sandbox};
use serde_json::{json, Value};
use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

const PROMPT: &str = "You are the manager inside Tirith. The owner chats with you here: answer briefly, in the \
owner's language. You do not write project code yourself. Tirith's autopilot starts the open items of each \
project's current plan step by itself (an agent per item, in its own git worktree); when an agent ends, Tirith \
runs the checks and merges its branch only when they pass. Your job: answer the lanes' questions \
(tirith answer), keep the plan good (clear items, and a check where one proves the item: \
`- [ ] Text (check: `cmd`)`), handle failed lanes and a red main, start or stop lanes by hand when needed, and \
ask the owner only for what only the owner can do, only with tirith ask-owner. Commands, run with your shell \
tool:\n  tirith status\n  tirith plan show <project>\n  tirith plan add <project> <step> \"<item text>\"\n  \
tirith lane start <project> <step> \"<item text>\"\n  tirith lane stop <lane>\n  tirith lane log <lane>\n  \
tirith answer <lane> \"<answer>\"\n  tirith ask-owner --kind credentials|payment|account|device|decision \"<question>\"\n  \
tirith autopilot on|off <project>\n  tirith room say <room> \"<text>\"\n\
Messages that start with [tirith] come from Tirith (questions of lanes, merges, failures, a red main, the \
owner's answers): act on them, but run tirith status first, because a message can be older than the state. A failed or stopped item is skipped by the autopilot until you restart it with \
tirith lane start (make the item clearer first if that is why it failed).";

fn user_line(text: &str) -> String {
    json!({"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": text}]}}).to_string()
}

/// Delivers a message to the session now, or when the next session starts.
fn deliver(app: &Shared, text: &str) {
    let line = user_line(text);
    let tx = app.manager_in.lock().unwrap().clone();
    match tx {
        Some(tx) if tx.send(line.clone()).is_ok() => {
            BUSY.store(true, Ordering::Relaxed);
            app.emit("busy", json!(true))
        }
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

/// True while a turn runs (between the owner's message and the session's `result`).
static BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// True while the manager works on a turn.
pub fn busy() -> bool {
    BUSY.load(Ordering::Relaxed)
}

/// Stops the current turn of the session (the session lives on): a stream-json `control_request` with subtype
/// `interrupt`, as the old collector sent. Err when no session runs or no turn is running.
pub fn interrupt(app: &Shared) -> Result<(), String> {
    if !BUSY.load(Ordering::Relaxed) {
        return Err("the manager is not working on a turn".into());
    }
    let tx = app.manager_in.lock().unwrap().clone().ok_or("no manager session is running")?;
    let id = format!("int-{:08x}", rand::random::<u32>());
    let line = json!({"type": "control_request", "request_id": id, "request": {"subtype": "interrupt"}}).to_string();
    tx.send(line).map_err(|_| "the manager session ended".to_string())
}

pub fn say(app: &Shared, text: &str) {
    app.chat("owner", text);
    deliver(app, text);
}

/// Notes from Tirith not yet handed to the manager.
static NOTES: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// A report from Tirith (a lane asked, merged, failed): shown in the chat at once and handed to the manager
/// together with every other note of the next 2 seconds, so many events cost one turn.
pub fn notify(app: &Shared, text: &str) {
    app.chat("tirith", text);
    tell(app, text);
}

/// Hands a report to the manager (gathered with the others of the next 2 s) without a chat line of its own.
pub fn tell(app: &Shared, text: &str) {
    let first = {
        let mut n = NOTES.lock().unwrap();
        n.push(text.to_string());
        n.len() == 1
    };
    if first {
        let app = app.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let notes = std::mem::take(&mut *NOTES.lock().unwrap());
            deliver(&app, &format!("[tirith] {}", notes.join("\n\n[tirith] ")));
        });
    }
}

const KINDS: [&str; 5] = ["credentials", "payment", "account", "device", "decision"];

/// `tirith ask-owner`: a card on the owner's page until answered.
pub fn ask_owner(app: &Shared, kind: &str, question: &str) -> Result<Value, String> {
    if !KINDS.contains(&kind) || question.trim().is_empty() {
        return Err(format!("ask the owner with --kind {} and a question", KINDS.join("|")));
    }
    let row = {
        let db = app.db.lock().unwrap();
        let n: i64 = db.query_row("SELECT COUNT(*) FROM owner_items", [], |r| r.get(0)).unwrap_or(0);
        let row = json!({"id": format!("o{}", n + 1), "at": now_iso(), "kind": kind, "question": question.trim(), "status": "open", "answer": ""});
        db.execute("INSERT INTO owner_items (id, at, kind, question, status, answer) VALUES (?1,?2,?3,?4,'open','')", rusqlite::params![row["id"].as_str(), row["at"].as_str(), kind, question.trim()])
            .map_err(|e| e.to_string())?;
        row
    };
    app.emit("owner", row.clone());
    Ok(row)
}

/// The owner's answer to an open card, handed to the manager.
pub fn owner_answer(app: &Shared, id: &str, answer: &str) -> Result<Value, String> {
    let kind: String = app.db.lock().unwrap().query_row("SELECT kind FROM owner_items WHERE id = ?1 AND status = 'open'", [id], |r| r.get(0)).map_err(|_| format!("no open question '{id}'"))?;
    let _ = app.db.lock().unwrap().execute("UPDATE owner_items SET status = 'answered', answer = ?1 WHERE id = ?2", rusqlite::params![answer, id]);
    app.emit("owner", json!({"id": id, "status": "answered", "answer": answer}));
    notify(app, &format!("The owner answered {id} ({kind}): {answer}"));
    Ok(json!({"ok": true}))
}

/// The owner's open cards, oldest first.
pub fn owner_open(app: &Shared) -> Vec<Value> {
    let db = app.db.lock().unwrap();
    db.prepare("SELECT id, at, kind, question FROM owner_items WHERE status = 'open' ORDER BY rowid")
        .and_then(|mut q| {
            q.query_map([], |r| Ok(json!({"id": r.get::<_, String>(0)?, "at": r.get::<_, String>(1)?, "kind": r.get::<_, String>(2)?, "question": r.get::<_, String>(3)?, "status": "open"})))
                .map(|rows| rows.flatten().collect())
        })
        .unwrap_or_default()
}

pub async fn run(app: Shared) {
    let mut backoff = 2;
    loop {
        let started = now_s();
        if let Err(e) = session(&app).await {
            app.event("", "", "error", "manager", &e, "");
        }
        *app.manager_in.lock().unwrap() = None;
        BUSY.store(false, Ordering::Relaxed);
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
    let mut command = launch.command(app);
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
                BUSY.store(false, Ordering::Relaxed);
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
