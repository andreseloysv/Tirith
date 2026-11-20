//! The managers: one long-lived agent session per project (and one for what belongs to no project); its text, tool calls and turn ends go straight to that project's chat.

use crate::app::{kill_group, now_iso, Shared};
use crate::{bots, models, rooms, rules, sandbox};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::{process::Stdio, sync::Mutex, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

/// The factory prompt of this conversation's role: a project's manager, or the lead (no project); a bot's own prompt wins.
fn prompt(app: &Shared, project: &str) -> String {
    let (role, p) = match app.cfg().project(project) {
        Some(p) => ("manager", Some(p.clone())),
        None => ("lead", None),
    };
    let text = app.cfg().factory.prompts_of(role);
    bots::prompt(app, project).unwrap_or_else(|| p.map_or(text.clone(), |p| text.replace("{id}", &p.id).replace("{repo}", &p.repo().display().to_string())))
}

/// One conversation: a project's (under its id) or the one of no project (under "").
#[derive(Default)]
struct Talk {
    /// The live session's stdin. Without one, `pending` waits for the session that `alive` says is on its way.
    tx: Option<mpsc::UnboundedSender<String>>,
    pending: Vec<String>,
    alive: bool,
    /// True while a turn runs (between a message handed to a live session and that session's `result`).
    busy: bool,
    pid: u32,
    /// The running turn is a one-shot exec run (Codex, Muse): interrupting it means killing it, not a control line.
    oneshot: bool,
    /// Rooms whose messages started the current turn (their answer goes back there) and the turn's text so far.
    reply_to: BTreeSet<String>,
    text: String,
    /// Reports from Tirith not yet handed over, and whether a hand-over is already on its way.
    notes: Vec<String>,
    due: bool,
    /// The current (or next) turn was started by the owner or by an event whose route has `phone = true`: its answer reaches the phones.
    phones: bool,
    /// The settings changed during a turn: the session ends when the turn does.
    stale: bool,
}

static TALKS: Mutex<BTreeMap<String, Talk>> = Mutex::new(BTreeMap::new());

fn talk<T>(project: &str, f: impl FnOnce(&mut Talk) -> T) -> T {
    f(TALKS.lock().unwrap().entry(project.to_string()).or_default())
}
fn set_busy(app: &Shared, project: &str, t: &mut Talk, on: bool) {
    t.busy = on;
    app.emit("busy", json!({"project": project, "on": on}));
}

/// The conversation of `project` (or of a bot): its own, or the one of no project when there is no such project (any more).
pub(crate) fn here<'a>(app: &Shared, project: &'a str) -> &'a str {
    if app.cfg().project(project).is_some() || bots::bot(app, project).is_some() {
        project
    } else {
        ""
    }
}

fn report(app: &Shared, project: &str, text: &str) {
    app.chat("tirith", text, project);
    rooms::to_phones(app, here(app, project), &format!("Tirith: {text}"));
}

fn known(app: &Shared, project: &str) -> Result<(), String> {
    (here(app, project) == project).then_some(()).ok_or(format!("unknown project '{project}'"))
}

/// Hands the conversation its waiting reports and `text` as one message; its session starts when none runs.
fn deliver(app: &Shared, project: &str, text: &str, phones: bool) {
    let memory = bots::context(app, project); // a bot's memory, read at every turn
    let start = talk(project, |t| {
        t.phones |= phones;
        if t.notes.is_empty() && text.is_empty() {
            return false;
        }
        let notes = std::mem::take(&mut t.notes).into_iter().map(|n| format!("[tirith] {n}"));
        let all: Vec<String> = memory.into_iter().chain(notes).chain((!text.is_empty()).then(|| text.to_string())).collect();
        let line = json!({"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": all.join("\n\n")}]}}).to_string();
        if !t.stale && t.tx.as_ref().is_some_and(|tx| tx.send(line.clone()).is_ok()) {
            set_busy(app, project, t, true);
        } else {
            t.pending.push(line); // the session on its way takes it, and only then is the manager busy
        }
        !std::mem::replace(&mut t.alive, true)
    });
    let _ = start.then(|| tokio::spawn(run(app.clone(), project.to_string())));
}

/// A message from a room (a paired phone or another agent): shown in the chat of the room's project, handed to that project's manager, and the manager's answer to this turn goes back to that room.
pub fn from_room(app: &Shared, project: &str, room: &str, author: &str, text: &str) {
    let project = here(app, project);
    app.event("", project, "room.message", "", &format!("{author} ({room}): {text}"), "");
    app.chat("room", &format!("{author} ({room}): {text}"), project);
    talk(project, |t| t.reply_to.insert(room.to_string()));
    deliver(app, project, &with_cards(app, project, &format!("[room {room}] {author}: {text}")), true);
}

/// What the owner wrote, and the project's cards still open: an answer in plain words is then closed by the manager.
fn with_cards(app: &Shared, project: &str, text: &str) -> String {
    let sql = "SELECT id, question FROM owner_items WHERE status = 'open' AND project = ?1 ORDER BY rowid";
    let open: Vec<String> = app.query(sql, [project], |r| Ok(format!("{}: {}", r.get::<_, String>(0)?, r.get::<_, String>(1)?.chars().take(160).collect::<String>())));
    if open.is_empty() {
        return text.to_string();
    }
    format!("{text}\n\n[tirith] Cards still open here: {}. If this answers one, close it with tirith answer <card> \"<the answer>\".", open.join(" | "))
}

/// Ends every session (the settings changed); one in the middle of a turn ends when the turn does, so its answer is not lost. Each one resumes its conversation at its next message.
pub fn restart() {
    TALKS.lock().unwrap().values_mut().for_each(|t| {
        t.stale = true;
        if !t.busy {
            t.tx.take();
            kill_group(t.pid, libc::SIGTERM);
        }
    });
}

pub fn busy() -> Vec<String> {
    TALKS.lock().unwrap().iter().filter(|(_, t)| t.busy).map(|(p, _)| p.clone()).collect()
}

/// Stops the current turn of the conversation's session (the session lives on): a stream-json `control_request` with subtype `interrupt`, as the old collector sent. Err when no turn is running.
pub fn interrupt(project: &str) -> Result<(), String> {
    let Some(()) = (!talk(project, |t| (t.oneshot && t.busy).then(|| kill_group(t.pid, libc::SIGTERM)).is_some())).then_some(()) else { return Ok(()) };
    let tx = talk(project, |t| t.tx.clone().filter(|_| t.busy)).ok_or("the manager is not working on a turn")?;
    let line = json!({"type": "control_request", "request_id": format!("int-{:08x}", rand::random::<u32>()), "request": {"subtype": "interrupt"}});
    tx.send(line.to_string()).map_err(|_| "the manager session ended".to_string())
}

pub fn say(app: &Shared, project: &str, text: &str) -> Result<(), String> {
    known(app, project)?;
    app.event("", project, "owner.message", "", text, "");
    app.chat("owner", text, project);
    if model_command(app, project, text) {
        return Ok(());
    }
    deliver(app, project, &with_cards(app, project, text), true);
    Ok(())
}

/// Only the local owner chat and authenticated paired phones call this path.
pub fn model_command(app: &Shared, project: &str, text: &str) -> bool {
    let Some(result) = models::command(app, text) else { return false };
    report(app, project, &result.unwrap_or_else(|e| format!("Model change failed: {e}")));
    true
}

/// The one way code emits an event: its row is kept and shown on the page, and every `[[routes]]` on it in the factory decides who hears it — a `chat` line (on the phones too with `phone`), and for its role a `turn` now or a `note` with its next turn. `told` is what the role is handed when it differs from the line shown. A turn's answer reaches the phones only when an event that started it had `phone` (or the owner wrote).
pub fn emit_event(app: &Shared, project: &str, on: &str, shown: &str, told: Option<&str>) {
    app.event("", project, on, "", shown, "");
    let routes = app.cfg().factory.routes_on(on);
    let here_p = here(app, project);
    if routes.iter().any(|r| r.2) {
        app.chat("tirith", shown, here_p);
        if routes.iter().any(|r| r.2 && r.3) {
            rooms::to_phones(app, here_p, &format!("Tirith: {shown}"));
        }
    }
    for (to, mode, _, phone, _) in routes {
        // Only chat conversations are handed text here; a turn role (the reviewer) is run by its own mechanism today.
        let dest = if to == "lead" {
            ""
        } else if to == "manager" {
            here_p
        } else {
            continue;
        };
        queue(app, dest, told.unwrap_or(shown), &mode, phone);
    }
}

/// A routed event for one conversation: with `turn` handed over (gathered with the other events of the next `limits.batch_seconds`, so many events cost one turn) and its `phone` marking that turn's answer; with `note` it waits for the conversation's next message. Anything else hears nothing.
fn queue(app: &Shared, project: &str, text: &str, mode: &str, phone: bool) {
    let turn = match mode {
        "turn" => true,
        "note" => false,
        _ => return,
    };
    let first = talk(project, |t| {
        t.notes.push(text.to_string());
        t.notes.drain(..t.notes.len().saturating_sub(30));
        t.phones |= turn && phone;
        turn && !std::mem::replace(&mut t.due, true)
    });
    if first {
        let (app, project, batch) = (app.clone(), project.to_string(), app.cfg().factory.limits.batch_seconds);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(batch)).await;
            talk(&project, |t| t.due = false);
            deliver(&app, &project, "", false);
        });
    }
}

/// A card's answer or closing, for the manager's next turn: a line in the project's chat and on the phones, and a note (no turn of its own).
fn note_line(app: &Shared, project: &str, text: &str) {
    report(app, project, text);
    queue(app, here(app, project), text, "note", false);
}

/// The manager's text of a turn, on the phones only when this turn's are marked (the owner wrote, or an event with `phone = true` started it).
fn answer_phones(app: &Shared, project: &str, text: &str) {
    if talk(project, |t| t.phones) {
        rooms::to_phones(app, project, text);
    }
}

/// `tirith ask-owner`: a card in its project's chat (so on the owner's phones too) and among the open cards of the owner's page, until answered. With `lane` it carries that waiting lane's question: the lane shows it, and the answer goes straight to the lane.
pub fn ask_owner(app: &Shared, project: &str, kind: &str, question: &str, lane: &str) -> Result<Value, String> {
    let project = match app.lane(lane) {
        Some(l) if l.status == "waiting" => l.project,
        _ if lane.is_empty() => project.to_string(),
        _ => return Err(format!("lane {lane} is not waiting for an answer")),
    };
    known(app, &project)?;
    let question = question.trim();
    let kinds = app.cfg().factory.words("cards", "kinds");
    (kinds.iter().any(|k| k == kind) && !question.is_empty()).then_some(()).ok_or_else(|| format!("ask the owner with --kind {} and a question", kinds.join("|")))?;
    let row = {
        let db = app.db.lock().unwrap();
        let n: i64 = db.query_row("SELECT COALESCE(MAX(CAST(SUBSTR(id, 2) AS INTEGER)), 0) FROM owner_items", [], |r| r.get(0)).unwrap_or(0);
        let row = json!({"id": format!("o{}", n + 1), "at": now_iso(), "kind": kind, "project": project, "question": question, "status": "open", "answer": "", "lane": lane});
        let sql = "INSERT INTO owner_items (id, at, kind, project, question, status, answer, lane) VALUES (?1,?2,?3,?4,?5,'open','',?6)";
        db.execute(sql, rusqlite::params![row["id"].as_str(), row["at"].as_str(), kind, project, question, lane]).map_err(|e| e.to_string())?;
        row
    };
    app.emit("owner", row.clone());
    let id = row["id"].as_str().unwrap_or("");
    emit_event(app, &project, "card.opened", &format!("{id} ({kind}): {question}"), None);
    app.chat("card", &format!("{id} ({kind}): {question}"), &project);
    rooms::to_phones(app, &project, &format!("❓ **Needs you** (card {id}, {kind})\n\n{question}\n\nAnswer here with `{id}: <your answer>`"));
    Ok(row)
}

/// The owner's answer to an open card: straight to the lane whose question it carries (the manager is told), else to the manager of the card's project with the question it answers. `by_manager`: the manager closes a card the owner answered in the chat.
pub fn owner_answer(app: &Shared, id: &str, answer: &str, by_manager: bool) -> Result<Value, String> {
    if let Some(l) = app.lane(id).filter(|l| l.status == "waiting") {
        crate::lanes::answer(app, id, answer, "the owner")?;
        note_line(app, &l.project, &format!("The owner answered lane {id} ({}) directly: {answer}", l.question));
        return Ok(json!({"ok": true}));
    }
    let sql = "SELECT kind, project, question, lane FROM owner_items WHERE id = ?1 AND status = 'open'";
    let (kind, project, question, lane): (String, String, String, String) = app.db.lock().unwrap().query_row(sql, [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).map_err(|_| format!("no open question '{id}'"))?;
    let _ = app.db.lock().unwrap().execute("UPDATE owner_items SET status = 'answered', answer = ?1 WHERE id = ?2", rusqlite::params![answer, id]);
    app.emit("owner", json!({"id": id, "status": "answered", "answer": answer}));
    let text = format!("The owner answered {id} ({kind}: {question}): {answer}");
    emit_event(app, &project, "card.answered", &text, None);
    if !lane.is_empty() && crate::lanes::answer(app, &lane, answer, "the owner").is_ok() {
        note_line(app, &project, &format!("{text}\nIt went straight to lane {lane}."));
    } else if by_manager {
        report(app, &project, &text);
    } else {
        report(app, &project, &text);
        queue(app, here(app, &project), &text, "turn", true);
    }
    Ok(json!({"ok": true}))
}

/// `tirith card close` (`by` the manager) or the card's Close (`by` the owner): an open card that is no longer needed is closed with its reason,
/// said in its project's chat and on the phones, so nobody is told to ignore it.
pub fn close_card(app: &Shared, id: &str, by: &str, why: &str) -> Result<Value, String> {
    let sql = "SELECT project FROM owner_items WHERE id = ?1 AND status = 'open'";
    let project: String = app.db.lock().unwrap().query_row(sql, [id], |r| r.get(0)).map_err(|_| format!("no open card '{id}'"))?;
    let note = format!("closed by {by}{}", if why.trim().is_empty() { String::new() } else { format!(": {}", why.trim()) });
    let _ = app.db.lock().unwrap().execute("UPDATE owner_items SET status = 'closed', answer = ?1 WHERE id = ?2", rusqlite::params![note, id]);
    app.emit("owner", json!({"id": id, "status": "closed", "answer": note}));
    emit_event(app, &project, "card.closed", &format!("Card {id} {note}"), None);
    note_line(app, &project, &format!("Card {id} {note}"));
    Ok(json!({"ok": true}))
}

/// A lane that asked with `tirith ask` is blocked until answered: it is an open question too.
pub fn waiting_cards(lanes: &[crate::app::Lane]) -> Vec<Value> {
    lanes
        .iter()
        .filter(|l| l.status == "waiting")
        .map(|l| json!({"id": l.id, "at": l.created_at, "kind": "lane", "project": l.project, "status": "open", "question": format!("lane bloqueada esperando al dueño ({}): {}", l.item, l.question)}))
        .collect()
}

pub fn owner_open(app: &Shared) -> Vec<Value> {
    let cols = ["id", "at", "kind", "question", "project", "status", "lane"];
    let cards = app.objects("SELECT id, at, kind, question, project, status, lane FROM owner_items WHERE status = 'open' ORDER BY rowid", [], &cols);
    // A lane whose question a card carries is that card.
    let carried: BTreeSet<&str> = cards.iter().filter_map(|c| c["lane"].as_str()).collect();
    let lanes: Vec<crate::app::Lane> = app.lanes.lock().unwrap().values().filter(|l| !carried.contains(l.id.as_str())).cloned().collect();
    [cards, waiting_cards(&lanes)].concat()
}

/// Why an open question no longer applies: it proposes turning its project's autopilot off (on) and that is already so.
pub fn stale_reason(question: &str, autopilot_on: bool, free_gb: f64) -> Option<&'static str> {
    let q = question.to_lowercase();
    if q.contains("gb free on this disk") && free_gb > 10.0 {
        return Some("the disk has more than 10 GB free again");
    }
    let has = |ws: &[&str]| ws.iter().any(|w| q.contains(w));
    let (off, on) = (has(&["turn off", "switch off", "disable", "apagar", "desactivar", "pausar"]), has(&["turn on", "switch on", "enable", "encender", "activar", "reanudar"]));
    match (q.contains("autopilot"), off, on, autopilot_on) {
        (true, true, false, false) => Some("the autopilot is already off"),
        (true, false, true, true) => Some("the autopilot is already on"),
        _ => None,
    }
}

pub fn close_stale_questions(app: &Shared) {
    for o in owner_open(app) {
        let (id, project, q) = (o["id"].as_str().unwrap_or(""), o["project"].as_str().unwrap_or(""), o["question"].as_str().unwrap_or(""));
        let lane = o["lane"].as_str().unwrap_or("");
        let moot = (!lane.is_empty() && !app.lane(lane).is_some_and(|l| l.status == "waiting")).then_some("its lane no longer waits for an answer");
        let Some(why) = moot.or_else(|| stale_reason(q, if project.is_empty() { app.cfg().autopilot } else { crate::lanes::autopilot_on(app, project) }, crate::lanes::free_gb(&app.home.join("lanes")))) else { continue };
        let answer = format!("closed by Tirith: {why}");
        let _ = app.db.lock().unwrap().execute("UPDATE owner_items SET status = 'closed', answer = ?1 WHERE id = ?2", rusqlite::params![answer, id]);
        app.emit("owner", json!({"id": id, "status": "closed", "answer": answer}));
    }
}

async fn run(app: Shared, project: String) {
    let mut backoff = 2;
    loop {
        let started = std::time::Instant::now();
        let result = session(&app, &project).await;
        if let Some(e) = result.as_ref().err().filter(|_| backoff == 2) {
            app.event("", &project, "error", "manager", e, "");
            report(&app, &project, &format!("The manager could not start: {e}. Tirith keeps trying; what you wrote waits for it."));
        }
        let rest = talk(&project, |t| {
            (t.tx, t.pid) = (None, 0);
            t.text.clear();
            set_busy(&app, &project, t, false);
            t.alive = !t.pending.is_empty();
            !t.alive
        });
        let Some(()) = (!rest).then_some(()) else { return };
        // A session that ended young is started again more and more slowly; one that lived is started again at once.
        backoff = if result.is_ok() || started.elapsed().as_secs() > 60 { 2 } else { (backoff * 2).min(60) };
        if backoff > 2 {
            tokio::time::sleep(Duration::from_secs(backoff)).await;
        }
    }
}

/// Starts a streaming chat agent of `role` ("manager" or "ceo") in `<home>/<role>` (a project's manager in its own folder below it, so the agent's own notes stay with the project), sandboxed, logging to `logs/<role>.log`.
pub fn spawn_chat(app: &Shared, role: &str, project: &str, argv: Vec<String>) -> Result<(tokio::process::Child, tokio::process::ChildStdin, tokio::process::ChildStdout), String> {
    let launch = sandbox::wrap(app, role, argv);
    let dir = app.home.join(role).join(project);
    let _ = std::fs::create_dir_all(&dir);
    let log = std::fs::OpenOptions::new().create(true).append(true).open(app.home.join(format!("logs/{role}.log"))).map_err(|e| e.to_string())?;
    let mut child = launch
        .command(app)
        .current_dir(&dir)
        .env("TIRITH_ROLE", role)
        .env("TIRITH_PROJECT", project)
        .env("TIRITH_HOME", &app.home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(log)
        .process_group(0)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("cannot start the {role} ({}): {e}", launch.program))?;
    let (stdin, stdout) = (child.stdin.take().ok_or("no stdin")?, child.stdout.take().ok_or("no stdout")?);
    Ok((child, stdin, stdout))
}

pub fn control_reply(msg: &Value, behavior: Value) -> String {
    json!({"type": "control_response", "response": {"subtype": "success", "request_id": msg["request_id"], "response": behavior}}).to_string()
}

/// One session of a conversation, until it ends or rests. Err when it never got ready: its messages wait again.
async fn session(app: &Shared, project: &str) -> Result<(), String> {
    let role = if project.starts_with("bot-") { project } else { "manager" }; // a bot may have a model of its own
    let alias = models::role(app, role).agent;
    if alias == "codex" || alias == "muse" {
        return exec_session(app, project, alias == "muse").await;
    }
    talk(project, |t| t.oneshot = false);
    let key = session_key(app, project, &alias);
    let resumed = app.kv(&key);
    let history = resumed.is_none().then(|| history(app, project));
    let mut cmd = models::chat_argv(app, role, resumed.as_deref())?;
    cmd.argv.extend(["--append-system-prompt".into(), prompt(app, project)]);
    let (mut child, mut stdin, stdout) = spawn_chat(app, "manager", project, cmd.argv)?;
    let pid = child.id().unwrap_or(0);
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let sent: Vec<String> = talk(project, |t| {
        (t.pid, t.tx, t.stale) = (pid, Some(tx.clone()), false);
        set_busy(app, project, t, true);
        t.pending
            .drain(..)
            .enumerate()
            .map(|(i, line)| {
                let mut v: Value = serde_json::from_str(&line).unwrap_or_default();
                let _ = history.as_ref().filter(|_| i == 0).and_then(|h| v["message"]["content"].as_array_mut().map(|a| a.insert(0, json!({"type": "text", "text": h}))));
                v.to_string()
            })
            .inspect(|line| drop(tx.send(line.clone())))
            .collect()
    });
    let writer = tokio::spawn(async move {
        while let Some(line) = rx.recv().await {
            if stdin.write_all(format!("{line}\n").as_bytes()).await.is_err() {
                break;
            }
            let _ = stdin.flush().await;
        }
    });
    let (mut ready, mut rest) = (false, false);
    let idle = Duration::from_secs(app.cfg().factory.role::<u64>("manager", "idle_minutes").unwrap_or(30) * 60);
    let mut lines = BufReader::new(stdout).lines();
    while !rest {
        let line = match tokio::time::timeout(idle, lines.next_line()).await {
            Ok(Ok(Some(line))) => line,
            Ok(_) => break,
            // Quiet for the role's idle_minutes and no turn running: the session rests until the next message resumes it.
            Err(_) => {
                rest = talk(project, |t| !t.busy && t.tx.take().is_some());
                continue;
            }
        };
        let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
        match msg["type"].as_str().unwrap_or("") {
            "system" if msg["subtype"] == "init" => {
                ready = true;
                msg["session_id"].as_str().into_iter().for_each(|id| app.set_kv(&key, Some(id)));
            }
            "assistant" => {
                for block in msg["message"]["content"].as_array().into_iter().flatten() {
                    match block["type"].as_str() {
                        Some("text") => {
                            let text = block["text"].as_str().unwrap_or("");
                            app.chat("manager", text, project);
                            answer_phones(app, project, text);
                            talk(project, |t| t.text = format!("{}\n\n{text}", t.text).trim_start().to_string());
                        }
                        Some("tool_use") => app.chat("tool", &rules::summary(block["name"].as_str().unwrap_or(""), &block["input"]), project),
                        _ => {}
                    }
                }
            }
            "result" => {
                let (rooms_now, text) = talk(project, |t| {
                    t.phones = false;
                    set_busy(app, project, t, false);
                    // A message that comes after this waits in `pending` for the next session, which has the new settings.
                    rest = std::mem::take(&mut t.stale) && t.tx.take().is_some();
                    (std::mem::take(&mut t.reply_to), std::mem::take(&mut t.text))
                });
                rooms::reply(app, &Vec::from_iter(rooms_now), &text);
            }
            "control_request" if msg["request"]["subtype"] == "can_use_tool" => {
                // The rules already ran in the PreToolUse hook; anything that reaches here is allowed.
                let _ = tx.send(control_reply(&msg, json!({"behavior": "allow", "updatedInput": msg["request"]["input"]})));
            }
            _ => {}
        }
    }
    // What a dead turn left is never sent to a room with a later answer; what rooms wrote for it is handed over again, once, and before the wait below, so a line that arrives during it (and waits in `pending`) is not handed twice.
    let died = talk(project, |t| {
        ready.then(|| t.reply_to.clear()); // a session that never got ready ran no turn: its lines wait, with their rooms
        t.tx.take().is_some() && ready && t.busy
    });
    let _ = died.then(|| rooms::again(app, Some(project)));
    writer.abort();
    kill_group(pid, libc::SIGTERM);
    let _ = tokio::time::timeout(Duration::from_secs(10), child.wait()).await;
    let Some(()) = (!ready && !rest).then_some(()) else { return Ok(()) };
    // It ended before it was ready: what it was handed waits again. A resume that does not work becomes a fresh session.
    talk(project, |t| drop(t.pending.splice(0..0, sent)));
    let Some(()) = resumed.is_some().then_some(()) else { return Err("its agent ended before it was ready (logs/manager.log says why)".into()) };
    app.set_kv(&key, None);
    Ok(())
}

fn session_key(app: &Shared, project: &str, agent: &str) -> String {
    let key = format!("manager_session:{agent}:{project}");
    let provider = format!("manager_agent:{project}");
    let legacy = (agent != "codex" && app.kv(&provider).is_none()).then(|| app.kv(&format!("manager_session{}{project}", if project.is_empty() { "" } else { ":" }))).flatten();
    let _ = (app.kv(&provider).as_deref() != Some(agent)).then(|| app.set_kv(&key, legacy.as_deref())).map(|_| app.set_kv(&provider, Some(agent)));
    key
}

fn history(app: &Shared, project: &str) -> String {
    let mut rows: Vec<String> = app.query("SELECT role, text FROM chat WHERE project = ?1 ORDER BY id DESC LIMIT 30", [project], |r| Ok(format!("{}: {}", r.get::<_, String>(0)?, r.get::<_, String>(1)?)));
    rows.reverse();
    format!("Recent conversation for context (do not repeat completed work):\n{}\n\nNew messages:\n", rows.join("\n"))
}

async fn exec_session(app: &Shared, project: &str, muse: bool) -> Result<(), String> {
    let key = session_key(app, project, if muse { "muse" } else { "codex" });
    let resumed = app.kv(&key);
    // The turn's message in plain text, with the recent conversation in front of it when the agent starts fresh; `sent` is handed over again when the turn never started.
    let sent = talk(project, |t| std::mem::take(&mut t.pending));
    let mut input = if resumed.is_none() { history(app, project) } else { String::new() };
    let texts: Vec<String> =
        sent.iter().filter_map(|l| serde_json::from_str::<Value>(l).ok()).flat_map(|v| v["message"]["content"].as_array().cloned().unwrap_or_default()).filter_map(|b| b["text"].as_str().map(|s| format!("{s}\n"))).collect();
    input += &texts.concat();
    let mut started = false;
    let result = crate::codex::turn(
        app,
        "manager",
        project,
        resumed.as_deref(),
        (&prompt(app, project), &input),
        |msg| {
            if muse {
                if let Some(id) = msg["stream"]["id"].as_str().filter(|_| msg["stream"]["kind"] == "session") {
                    app.set_kv(&key, Some(id))
                }
                if msg["payload_type"] == "tool.result" {
                    // The shell's own result text is its result JSON: the command it ran is the line worth showing.
                    let detail = msg["payload"]["text"].as_str().and_then(|s| serde_json::from_str::<Value>(s).ok()).and_then(|v| v["command"].as_str().map(String::from));
                    app.chat("tool", detail.as_deref().unwrap_or_else(|| msg["payload"]["correlation_facts"]["tool_name"].as_str().unwrap_or("tool")), project);
                }
                return;
            }
            match msg["type"].as_str().unwrap_or("") {
                "thread.started" => {
                    if let Some(id) = msg["thread_id"].as_str() {
                        app.set_kv(&key, Some(id))
                    }
                }
                "item.completed" if msg["item"]["type"] == "agent_message" => {
                    let text = msg["item"]["text"].as_str().unwrap_or("");
                    app.chat("manager", text, project);
                    answer_phones(app, project, text);
                }
                "item.started" if msg["item"]["type"] == "command_execution" => app.chat("tool", msg["item"]["command"].as_str().unwrap_or("command"), project),
                _ => {}
            }
        },
        |pid| {
            started = true;
            talk(project, |t| {
                (t.pid, t.oneshot, t.stale) = (pid, true, false);
                set_busy(app, project, t, true);
            });
        },
    )
    .await;
    if !started {
        talk(project, |t| drop(t.pending.splice(0..0, sent)));
        return result.map(|_| ());
    }
    let (rooms_now, phones) = talk(project, |t| {
        set_busy(app, project, t, false);
        (std::mem::take(&mut t.reply_to), std::mem::take(&mut t.phones))
    });
    let (text, answered) = match result {
        Ok(text) => (text, true),
        Err(e) => {
            let text = format!("{}: {e}. Send your message again to retry.", if muse { "The muse agent failed" } else { "Codex" });
            report(app, project, &text);
            app.set_kv(&key, None);
            (text, false)
        }
    };
    // Codex speaks while it works; an exec turn of the other agent answers once, at its end.
    let _ = (muse && answered).then(|| app.chat("manager", &text, project)).map(|_| phones.then(|| rooms::to_phones(app, project, &text)));
    rooms::reply(app, &Vec::from_iter(rooms_now), &text);
    Ok(())
}

#[cfg(test)]
#[path = "../checks/manager.rs"]
mod tests;
