//! The managers: one long-lived agent session per project (and one for what belongs to no project); its text, tool calls and turn ends go straight to that project's chat.
use crate::app::{kill_group, now_iso, now_s, Shared};
use crate::{bots, models, rooms, rules, sandbox};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::{process::Stdio, sync::Mutex, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
/// The factory prompt of this conversation's role (a bot's is its role's, with the shared skills folder filled in).
fn prompt(app: &Shared, project: &str) -> String {
    let text = app.cfg().factory.prompts_of(&app.chat_role(project)).replace("{shared}", &app.home.join("bots/skills").display().to_string());
    app.cfg().project(project).map_or(text.clone(), |p| text.replace("{id}", &p.id).replace("{repo}", &p.repo().display().to_string()))
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
    /// Rooms whose messages wait for the next turn (`reply_to`, their answer goes back there), the rooms of the turn now running (`turn_rooms`: only what they were owed when it started stops being owed at its end) and the turn's text so far.
    reply_to: BTreeSet<String>,
    turn_rooms: BTreeSet<String>,
    text: String,
    /// Reports from Tirith not yet handed over, and whether a hand-over is already on its way.
    notes: Vec<String>,
    due: bool,
    /// The current (or next) turn was started by the owner or by an event whose route has `phone = true`: its answer reaches the phones.
    phones: bool,
    /// The settings changed during a turn: the session ends when the turn does.
    stale: bool,
    /// The current turn's live progress for the phones in its rooms (see `rooms::Typing`); idle when the turn has no rooms.
    typing: rooms::Typing,
}
static TALKS: Mutex<BTreeMap<String, Talk>> = Mutex::new(BTreeMap::new());
fn talk<T>(project: &str, f: impl FnOnce(&mut Talk) -> T) -> T {
    f(TALKS.lock().unwrap().entry(project.to_string()).or_default())
}
fn set_busy(app: &Shared, project: &str, t: &mut Talk, on: bool) {
    t.busy = on;
    app.emit("busy", json!({"project": project, "on": on}));
}
/// The rooms a starting turn shows its progress in: the rooms that started it, and, for a turn marked for the phones, its conversation's own rooms. A turn with neither (only the page) writes in no room.
fn typing_rooms(app: &Shared, project: &str) -> Vec<String> {
    let (reply_to, phones) = talk(project, |t| (t.reply_to.iter().chain(t.turn_rooms.iter()).cloned().collect::<BTreeSet<_>>(), t.phones));
    reply_to.into_iter().chain(phones.then(|| rooms::phone_rooms(app, project)).into_iter().flatten()).collect::<BTreeSet<_>>().into_iter().collect()
}
/// Shows a turn's typing in its rooms: at its start, and with `again` (another message for the running turn, whose phones may have been reopened) its current state once more, in every room of the turn by then.
fn typing_start(app: &Shared, project: &str, again: bool) {
    let rooms = typing_rooms(app, project);
    rooms::typing(talk(project, |t| t.typing.start(rooms, now_s(), again)));
}
/// Every busy turn's heartbeat (called by the rooms loop, about every 30 s): due ones resend their current state to their rooms alone, the rest send nothing; a stopped turn has no rooms left, so its heartbeat is over with it.
pub fn typing_heartbeats() {
    // Sent under the talk's lock, so a heartbeat that was due as its turn ended can never land after that turn's typing off.
    busy().iter().for_each(|p| talk(p, |t| rooms::typing(t.typing.heartbeat(now_s()))));
}
fn typing_line(project: &str, line: &str) {
    rooms::typing(talk(project, |t| t.typing.line(line, now_s())));
}
fn typing_stop(project: &str) {
    rooms::typing(talk(project, |t| t.typing.stop()));
}
/// A turn starts (a live session took its message, a session took its waiting ones, or an exec turn took its input): its rooms are the ones waiting by then, and only what they were owed at this moment stops being owed when it ends.
fn begin_turn(app: &Shared, project: &str) {
    let rooms: Vec<String> = talk(project, |t| t.turn_rooms.is_empty().then(|| (t.turn_rooms.append(&mut t.reply_to), Vec::from_iter(t.turn_rooms.clone())).1).into_iter().flatten().collect());
    rooms::begin(app, &rooms);
}
/// The short line a tool call shows on the phones: its own summary (a description, a file, a pattern, a url) when it has one, never its raw command; without one, only the tool's name.
fn progress_line(tool: &str, input: &Value) -> String {
    let detail = ["description", "file_path", "notebook_path", "pattern", "url", "prompt", "skill"].iter().find_map(|k| input[*k].as_str()).unwrap_or("");
    let short = detail.lines().next().unwrap_or("").trim();
    Some(short).filter(|s| !s.is_empty()).map_or_else(|| tool.to_string(), |short| format!("{}{short}", if tool.is_empty() { String::new() } else { format!("{tool}: ") }))
}
fn report(app: &Shared, project: &str, text: &str) {
    app.chat("tirith", text, project);
    rooms::to_phones(app, app.conv(project), &format!("Tirith: {text}"));
}
/// One user message as a stream-json line. `pending` keeps plain texts; only a live session's stdin sees lines.
fn user_line(text: &str) -> String {
    json!({"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": text}]}}).to_string()
}
/// Hands the conversation its waiting reports and `text` as one message; its session starts when none runs.
fn deliver(app: &Shared, project: &str, text: &str, phones: bool) {
    let memory = bots::context(app, project); // a bot's memory, read at every turn
    let (start, handed, began) = talk(project, |t| {
        let was_busy = (t.phones |= phones, t.busy).1;
        let Some(()) = (!t.notes.is_empty() || !text.is_empty()).then_some(()) else { return (false, false, false) };
        let notes = std::mem::take(&mut t.notes).into_iter().map(|n| format!("[tirith] {n}"));
        let all: Vec<String> = memory.into_iter().chain(notes).chain((!text.is_empty()).then(|| text.to_string())).collect();
        let text = all.join("\n\n");
        let sent = !t.stale && t.tx.as_ref().is_some_and(|tx| tx.send(user_line(&text)).is_ok());
        if sent {
            set_busy(app, project, t, true);
        } else {
            t.pending.push(text); // the session on its way takes it, and only then is the manager busy
        }
        (!std::mem::replace(&mut t.alive, true), true, sent && !was_busy)
    });
    let _ = began.then(|| begin_turn(app, project)); // a message a live, idle session took starts its turn now; one for a running turn begins at that turn's end
    let _ = handed.then(|| typing_start(app, project, true)); // the phones see the turn start (or, for a turn already running, its state again), even while its session is still on its way
    let _ = start.then(|| tokio::spawn(run(app.clone(), project.to_string())));
}
/// A message from a room (a paired phone or another agent): shown in the chat of the room's project, handed to that project's manager, and the manager's answer to this turn goes back to that room.
pub fn from_room(app: &Shared, project: &str, room: &str, author: &str, text: &str) {
    let project = app.conv(project);
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
/// Ends a conversation's live session at its turn's end (now, when no turn runs), so its next message starts a fresh one and a running turn's answer is not lost.
fn end(t: &mut Talk) {
    t.stale = true;
    let _ = (!t.busy).then(|| (t.tx.take(), kill_group(t.pid, libc::SIGTERM)));
}
/// Ends one conversation's live session (its chat was cleared), like `restart` for one.
pub fn end_session(project: &str) {
    talk(project, end);
}
/// Ends every session (the settings changed); each one resumes its conversation at its next message.
pub fn restart() {
    TALKS.lock().unwrap().values_mut().for_each(end);
}
pub fn busy() -> Vec<String> {
    TALKS.lock().unwrap().iter().filter(|(_, t)| t.busy).map(|(p, _)| p.clone()).collect()
}
/// Stops the current turn of the conversation's session (the session lives on): a stream-json `control_request` with subtype `interrupt`, as the old collector sent. Err when no turn is running.
pub fn interrupt(project: &str) -> Result<(), String> {
    // A busy turn without a live session's stdin is a one-shot exec run (Codex, Muse): interrupting it means killing it, not a control line.
    let Some(()) = (!talk(project, |t| (t.busy && t.tx.is_none()).then(|| kill_group(t.pid, libc::SIGTERM)).is_some())).then_some(()) else { return Ok(()) };
    let tx = talk(project, |t| t.tx.clone().filter(|_| t.busy)).ok_or("the manager is not working on a turn")?;
    tx.send(json!({"type": "control_request", "request_id": format!("int-{:08x}", rand::random::<u32>()), "request": {"subtype": "interrupt"}}).to_string()).map_err(|_| "the manager session ended".to_string())
}
/// What `tirith say` and the page's chat write. `agent` is the role or lane of the Tirith session it ran in (`TIRITH_ROLE`/`TIRITH_LANE`), empty for the owner: an agent's words go in the chat under its name and to the manager marked as its, never as an owner message (no `owner.message`, no owner-mood eval, no `/model`, which is the owner's alone).
pub fn say(app: &Shared, project: &str, text: &str, agent: &str) -> Result<(), String> {
    (project.is_empty() || app.conv(project) == project).then_some(()).ok_or(format!("unknown project '{project}'"))?;
    let project = app.conv(project);
    let _ = (!agent.is_empty()).then(|| (app.chat(agent, text, project), deliver(app, project, &format!("[agent {agent}] {text}"), false)));
    let Some(()) = agent.is_empty().then_some(()) else { return Ok(()) };
    let event = app.event("", project, "owner.message", "", text, "");
    app.chat("owner", text, project);
    judge_evals(app, project, "owner.message", text, event);
    let _ = (!model_command(app, project, text)).then(|| deliver(app, project, &with_cards(app, project, text), true));
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
    let event = app.event("", project, on, "", shown, "");
    let routes = app.cfg().factory.routes_on(on);
    let here_p = app.conv(project);
    if routes.iter().any(|r| r.2 && r.3) {
        report(app, here_p, shown);
    } else if routes.iter().any(|r| r.2) {
        app.chat("tirith", shown, here_p);
    }
    let mut heard: BTreeSet<String> = BTreeSet::new();
    for (to, mode, _, phone, _) in routes {
        // Only chat roles are handed text here, in the conversation of their scope; a turn role is run by whoever watches its condition (the metrics run the CEO through `run_turn`, like any turn role).
        let f = &app.cfg().factory;
        let Some(()) = (f.role::<String>(&to, "session").as_deref() == Some("chat")).then_some(()) else { continue };
        // A due routine (its bot's own `schedule` route) is for that bot's conversation alone, and a conversation that more than one route reaches (the factory's schedule route and the bot's) hears the event once.
        let Some(()) = (!(on == "schedule" && f.role::<String>(&to, "scope").as_deref() == Some("own") && to != here_p)).then_some(()) else { continue };
        let dest = if f.role::<String>(&to, "scope").as_deref() == Some("own") { &to } else { here_p };
        let _ = heard.insert(dest.to_string()).then(|| queue(app, dest, told.unwrap_or(shown), &mode, phone));
    }
    judge_evals(app, project, on, shown, event);
}
/// The card an event's line names (`Card o12 …`, `The owner answered o12 …`): its first word shaped like one.
pub fn card_id_of(shown: &str) -> Option<String> {
    shown.split(|c: char| !c.is_ascii_alphanumeric()).find(|w| w.len() > 1 && w.starts_with('o') && w[1..].bytes().all(|b| b.is_ascii_digit())).map(String::from)
}
/// A judge's answer read as (label, score, why): its first word, a number when the rubric asks for a score from 0 to 1, and the rest of its first line.
fn judged(text: &str) -> (String, Option<f64>, String) {
    let line = text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or_default();
    let head = line.split_whitespace().next().unwrap_or_default().trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '-').to_lowercase();
    let why = line[line.to_lowercase().find(&head).map(|i| i + head.len()).unwrap_or(0)..].trim_start_matches(|c: char| c.is_whitespace() || "—–-|:,.;".contains(c)).to_string();
    match head.parse::<f64>() {
        Ok(n) if (0.0..=1.0).contains(&n) => (String::new(), Some(n), why),
        _ => (head, None, why),
    }
}
/// Every `[[evals]]` on this event, judged in the background by its judge role (a cheap turn role whose instructions are the rubric): the event with the conversation before it (and, for a card, the card itself) goes in, and a `scores` row — the label or score, one line why, the event, the eval and the judge's factory stamp — is kept. Nothing reaches the owner: the scores only feed the measures. A share of the events is judged (`sample`, by event id, so a restart judges no event twice for that share).
fn judge_evals(app: &Shared, project: &str, on: &str, shown: &str, event: i64) {
    let evals: Vec<_> = app.cfg().factory.evals_on(on).into_iter().filter(|e| event.rem_euclid(100) < (e.3 * 100.0) as i64).collect();
    let Some(()) = (!evals.is_empty() && tokio::runtime::Handle::try_current().is_ok()).then_some(()) else { return };
    let (app, project, shown) = (app.clone(), app.conv(project).to_string(), shown.to_string());
    tokio::spawn(async move {
        let card = card_id_of(&shown).and_then(|id| {
            app.query("SELECT kind, question, status, answer FROM owner_items WHERE id = ?1", [&id], |r| {
                Ok(format!("Card {id} ({}, {}): {}\nIts answer or closing note: {}", r.get::<_, String>(0)?, r.get::<_, String>(2)?, r.get::<_, String>(1)?, r.get::<_, String>(3)?))
            })
            .into_iter()
            .next()
        });
        let input = format!("{}\nThe event to judge: {shown}\n{}", history(&app, &project), card.map(|c| format!("\n{c}\n")).unwrap_or_default());
        for (id, judge, rubric, _) in evals {
            let Ok(text) = run_turn(&app, &judge, &project, &format!("{}\n\n{input}", app.cfg().factory.prompt(&rubric))).await else { continue };
            let (label, score, why) = judged(&text);
            let (factory, role_hash) = app.stamp_for(&judge);
            app.insert("INSERT INTO scores (at, event, eval, label, score, why, factory, role_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", rusqlite::params![now_iso(), event, id, label, score, why, factory, role_hash]);
        }
    });
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
    queue(app, app.conv(project), text, "note", false);
}
/// Sets a card's final state in the table and on the page.
fn card_done(app: &Shared, id: &str, status: &str, answer: &str) {
    let _ = app.db.lock().unwrap().execute("UPDATE owner_items SET status = ?1, answer = ?2 WHERE id = ?3", rusqlite::params![status, answer, id]);
    app.emit("owner", json!({"id": id, "status": status, "answer": answer}));
}
/// `tirith ask-owner`: a card in its project's chat (so on the owner's phones too) and among the open cards of the owner's page, until answered. With `lane` it carries that waiting lane's question: the lane shows it, and the answer goes straight to the lane.
pub fn ask_owner(app: &Shared, project: &str, kind: &str, question: &str, lane: &str) -> Result<Value, String> {
    let project = match app.lane(lane) {
        Some(l) if l.status == "waiting" => l.project,
        _ if lane.is_empty() => project.to_string(),
        _ => return Err(format!("lane {lane} is not waiting for an answer")),
    };
    (project.is_empty() || app.conv(&project) == project).then_some(()).ok_or(format!("unknown project '{project}'"))?;
    let project = app.conv(&project).to_string();
    let question = question.trim();
    let kinds = app.cfg().factory.words("cards", "kinds");
    (kinds.iter().any(|k| k == kind) && !question.is_empty()).then_some(()).ok_or_else(|| format!("ask the owner with --kind {} and a question", kinds.join("|")))?;
    let (factory, role_hash) = app.lane(lane).map(|l| (l.factory, l.role_hash)).unwrap_or_else(|| app.stamp_for(&app.chat_role(&project))); // a lane's card carries its lane's stamp, else its conversation role's
    let row = {
        let db = app.db.lock().unwrap();
        let n: i64 = db.query_row("SELECT COALESCE(MAX(CAST(SUBSTR(id, 2) AS INTEGER)), 0) FROM owner_items", [], |r| r.get(0)).unwrap_or(0);
        let row = json!({"id": format!("o{}", n + 1), "at": now_iso(), "kind": kind, "project": project, "question": question, "status": "open", "answer": "", "lane": lane, "factory": factory, "role_hash": role_hash});
        let sql = "INSERT INTO owner_items (id, at, kind, project, question, status, answer, lane, factory, role_hash) VALUES (?1,?2,?3,?4,?5,'open','',?6,?7,?8)";
        db.execute(sql, rusqlite::params![row["id"].as_str(), row["at"].as_str(), kind, project, question, lane, row["factory"].as_str(), row["role_hash"].as_str()]).map_err(|e| e.to_string())?;
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
    card_done(app, id, "answered", answer);
    let text = format!("The owner answered {id} ({kind}: {question}): {answer}");
    emit_event(app, &project, "card.answered", &text, None);
    if !lane.is_empty() && crate::lanes::answer(app, &lane, answer, "the owner").is_ok() {
        note_line(app, &project, &format!("{text}\nIt went straight to lane {lane}."));
    } else {
        report(app, &project, &text);
        let _ = (!by_manager).then(|| queue(app, app.conv(&project), &text, "turn", true));
    }
    Ok(json!({"ok": true}))
}
/// `tirith card close` (`by` the manager) or the card's Close (`by` the owner): an open card that is no longer needed is closed with its reason, said in its project's chat and on the phones, so nobody is told to ignore it.
pub fn close_card(app: &Shared, id: &str, by: &str, why: &str) -> Result<Value, String> {
    let sql = "SELECT project FROM owner_items WHERE id = ?1 AND status = 'open'";
    let project: String = app.db.lock().unwrap().query_row(sql, [id], |r| r.get(0)).map_err(|_| format!("no open card '{id}'"))?;
    let note = format!("closed by {by}{}", if why.trim().is_empty() { String::new() } else { format!(": {}", why.trim()) });
    card_done(app, id, "closed", &note);
    emit_event(app, &project, "card.closed", &format!("Card {id} {note}"), None);
    note_line(app, &project, &format!("Card {id} {note}"));
    Ok(json!({"ok": true}))
}
/// A lane that asked with `tirith ask` is blocked until answered: it is an open question too.
pub fn waiting_cards(lanes: &[crate::app::Lane]) -> Vec<Value> {
    lanes
        .iter()
        .filter(|l| l.status == "waiting")
        .map(|l| json!({"id": l.id, "at": l.created_at, "kind": "lane", "project": l.project, "status": "open", "factory": l.factory, "role_hash": l.role_hash, "question": format!("lane bloqueada esperando al dueño ({}): {}", l.item, l.question)}))
        .collect()
}
pub fn owner_open(app: &Shared) -> Vec<Value> {
    let cols = ["id", "at", "kind", "question", "project", "status", "lane", "factory", "role_hash"];
    let cards = app.objects("SELECT id, at, kind, question, project, status, lane, factory, role_hash FROM owner_items WHERE status = 'open' ORDER BY rowid", [], &cols);
    // A lane whose question a card carries is that card.
    let carried: BTreeSet<&str> = cards.iter().filter_map(|c| c["lane"].as_str()).collect();
    let lanes: Vec<crate::app::Lane> = app.lanes.lock().unwrap().values().filter(|l| !carried.contains(l.id.as_str())).cloned().collect();
    [cards, waiting_cards(&lanes)].concat()
}
/// Why an open question no longer applies: it proposes turning its project's autopilot off (on) and that is already so.
pub fn stale_reason(question: &str, autopilot_on: bool, free_gb: f64) -> Option<&'static str> {
    let q = question.to_lowercase();
    let Some(()) = (!(q.contains("gb free on this disk") && free_gb > 10.0)).then_some(()) else { return Some("the disk has more than 10 GB free again") };
    let has = |ws: &[&str]| ws.iter().any(|w| q.contains(w));
    let (off, on) = (has(&["turn off", "switch off", "disable", "apagar", "desactivar", "pausar"]), has(&["turn on", "switch on", "enable", "encender", "activar", "reanudar"]));
    match (q.contains("autopilot"), off, on, autopilot_on) {
        (true, true, false, false) => Some("the autopilot is already off"),
        (true, false, true, true) => Some("the autopilot is already on"),
        _ => None,
    }
}
pub fn close_stale_questions(app: &Shared) {
    for mut o in owner_open(app) {
        let (id, project, q, lane) = (o["id"].as_str().unwrap_or("").to_string(), o["project"].as_str().unwrap_or("").to_string(), o["question"].as_str().unwrap_or("").to_string(), o["lane"].as_str().unwrap_or("").to_string());
        if !lane.is_empty() && !app.lane(&lane).is_some_and(|l| l.status == "waiting") {
            // Its lane no longer waits (answered, merged, closed): the card stays open for the owner as a card of its project, same id, kind and question; only a card of no lane can go stale.
            let _ = app.db.lock().unwrap().execute("UPDATE owner_items SET lane = '' WHERE id = ?1 AND status = 'open'", rusqlite::params![id]);
            o["lane"] = json!("");
            app.emit("owner", o);
        } else if lane.is_empty() {
            stale_reason(&q, if project.is_empty() { app.cfg().autopilot } else { crate::lanes::autopilot_on(app, &project) }, crate::lanes::free_gb(&app.home.join("lanes")))
                .into_iter()
                .for_each(|why| card_done(app, &id, "closed", &format!("closed by Tirith: {why}")));
        }
    }
}
async fn run(app: Shared, project: String) {
    let mut backoff = 2;
    loop {
        let started = std::time::Instant::now();
        let result = session(&app, &project).await;
        result
            .as_ref()
            .err()
            .filter(|_| backoff == 2)
            .into_iter()
            .for_each(|e| (app.event("", &project, "error", "manager", e, ""), report(&app, &project, &format!("The manager could not start: {e}. Tirith keeps trying; what you wrote waits for it."))).1);
        typing_stop(&project); // a session that ended, failed or never got ready leaves no turn typing on the phones
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
        tokio::time::sleep(Duration::from_secs(if backoff > 2 { backoff } else { 0 })).await;
    }
}
/// Starts a streaming chat agent on behalf of `role` in `<home>/<role>` (a project's chat in its own folder below it, so the agent's own notes stay with the project), sandboxed, logging to `logs/<role>.log`. A bot's own turn role (the CEO, the judge) runs in its bot folder `<home>/manager/<role>`, like its conversation does.
pub fn spawn_chat(app: &Shared, role: &str, project: &str, argv: Vec<String>) -> Result<(tokio::process::Child, tokio::process::ChildStdin, tokio::process::ChildStdout), String> {
    let launch = sandbox::wrap(app, role, argv);
    let dir = if app.cfg().factory.role::<String>(role, "scope").as_deref() == Some("own") { app.home.join("manager").join(role) } else { app.home.join(role).join(project) };
    let _ = std::fs::create_dir_all(&dir);
    let log = std::fs::OpenOptions::new().create(true).append(true).open(app.home.join(format!("logs/{role}.log"))).map_err(|e| e.to_string())?;
    let mut child = launch
        .command(app)
        .current_dir(&dir)
        .env("TIRITH_ROLE", role)
        .env("TIRITH_PROJECT", project)
        .env("CARGO_TARGET_DIR", crate::lanes::shared_target(app, if project.is_empty() { role } else { project })) // managers, bots and the CEO build into the shared cache: a cargo of theirs in a project folder leaves no target/ there
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
/// A tool call of a turn: with `access = "read"` the file-writing tools are denied and everything else (including every `tirith` command that only reads) is allowed; any other access allows all.
fn turn_permission(read_only: bool, tool: &str, input: &Value) -> Value {
    if read_only && crate::rules::EDIT_TOOLS.contains(&tool) {
        json!({"behavior": "deny", "message": "this role is read-only (access = \"read\")"})
    } else {
        json!({"behavior": "allow", "updatedInput": input})
    }
}
/// One fresh turn of a `session = "turn"` role: its factory prompt as its instructions, `input` as its one message, its `access` deciding whether it may write; the assistant's text until the turn ends is the answer. Any turn role runs this way (the CEO included).
pub async fn run_turn(app: &Shared, role: &str, project: &str, input: &str) -> Result<String, String> {
    let prompt = app.cfg().factory.prompts_of(role);
    let Some(()) = (!matches!(models::role(app, role).agent.as_str(), "codex" | "muse")).then_some(()) else { return crate::codex::turn(app, role, project, None, (&prompt, input), |_| {}, |_| {}).await };
    let read_only = app.cfg().factory.role::<String>(role, "access").as_deref() == Some("read");
    let mut cmd = models::chat_argv(app, role, None)?;
    cmd.argv.extend(["--append-system-prompt".into(), prompt]);
    let (mut child, mut stdin, stdout) = spawn_chat(app, role, project, cmd.argv)?;
    let pid = child.id().unwrap_or(0);
    let mut lines = BufReader::new(stdout).lines();
    let send = |line: String| (line + "\n").into_bytes();
    stdin.write_all(&send(user_line(input))).await.map_err(|e| e.to_string())?;
    let mut text = String::new();
    let read = async {
        while let Ok(Some(line)) = lines.next_line().await {
            let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
            match msg["type"].as_str().unwrap_or("") {
                "assistant" => msg["message"]["content"].as_array().into_iter().flatten().filter(|b| b["type"] == "text").for_each(|b| text += &format!("{}\n", b["text"].as_str().unwrap_or(""))),
                "result" => return true,
                "control_request" if msg["request"]["subtype"] == "can_use_tool" => {
                    let behavior = turn_permission(read_only, msg["request"]["tool_name"].as_str().unwrap_or(""), &msg["request"]["input"]);
                    let _ = stdin.write_all(&send(control_reply(&msg, behavior))).await;
                }
                _ => {}
            }
        }
        false
    };
    let finished = tokio::time::timeout(Duration::from_secs(600), read).await;
    kill_group(pid, libc::SIGKILL);
    let _ = child.wait().await;
    let text = text.trim().to_string();
    finished
        .map_err(|_| format!("no answer within 10 minutes{}", if text.is_empty() { String::new() } else { format!(": {text}") }))
        .and_then(|_| (!text.is_empty()).then_some(text).ok_or_else(|| "the session ended without an answer".to_string()))
}
/// One session of a conversation, until it ends or rests. Err when it never got ready: its messages wait again.
async fn session(app: &Shared, project: &str) -> Result<(), String> {
    let role = app.chat_role(project); // a bot may have a model of its own, under its key
    let alias = models::role(app, &role).agent;
    let Some(()) = (!matches!(alias.as_str(), "codex" | "muse")).then_some(()) else { return exec_session(app, project, alias == "muse").await };
    let key = format!("manager_session:{alias}:{project}");
    let resumed = app.kv(&key);
    let history = resumed.is_none().then(|| history(app, project));
    let mut cmd = models::chat_argv(app, &role, resumed.as_deref())?;
    cmd.argv.extend(["--append-system-prompt".into(), prompt(app, project)]);
    let (mut child, mut stdin, stdout) = spawn_chat(app, "manager", project, cmd.argv)?;
    let pid = child.id().unwrap_or(0);
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let sent: Vec<String> = talk(project, |t| {
        (t.pid, t.tx, t.stale) = (pid, Some(tx.clone()), false);
        set_busy(app, project, t, true);
        let sent: Vec<String> = std::mem::take(&mut t.pending);
        sent.iter().enumerate().for_each(|(i, text)| drop(tx.send(user_line(&if i == 0 { format!("{}{text}", history.as_deref().unwrap_or("")) } else { text.clone() }))));
        sent
    });
    let _ = (!sent.is_empty()).then(|| (begin_turn(app, project), typing_start(app, project, false))); // the waiting turn starts now; a session restarted after a failure shows it again
    let writer = tokio::spawn(async move {
        while let Some(line) = rx.recv().await {
            let Some(()) = stdin.write_all(format!("{line}\n").as_bytes()).await.is_ok().then_some(()) else { break };
            let _ = stdin.flush().await;
        }
    });
    let (mut ready, mut rest) = (false, false);
    let idle = Duration::from_secs(app.cfg().factory.role::<u64>(&role, "idle_minutes").unwrap_or(30) * 60);
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
            "assistant" => msg["message"]["content"].as_array().into_iter().flatten().for_each(|block| match block["type"].as_str() {
                Some("text") => {
                    let text = block["text"].as_str().unwrap_or("");
                    app.chat("manager", text, project);
                    let _ = talk(project, |t| t.phones).then(|| rooms::to_phones(app, project, text)); // on the phones only when this turn is marked (owner, or phone = true)
                    talk(project, |t| t.text = format!("{}\n\n{text}", t.text).trim_start().to_string());
                }
                Some("tool_use") => {
                    app.chat("tool", &rules::summary(block["name"].as_str().unwrap_or(""), &block["input"]), project);
                    typing_line(project, &progress_line(block["name"].as_str().unwrap_or(""), &block["input"]));
                }
                _ => {}
            }),
            "result" => {
                let (rooms_now, text) = talk(project, |t| {
                    t.phones = false;
                    set_busy(app, project, t, false);
                    // A message that comes after this waits in `pending` for the next session, which has the new settings.
                    rest = std::mem::take(&mut t.stale) && t.tx.take().is_some();
                    (std::mem::take(&mut t.turn_rooms), std::mem::take(&mut t.text))
                });
                typing_stop(project);
                rooms::reply(app, &Vec::from_iter(rooms_now), &text);
                let _ = (!rest).then(|| begin_turn(app, project)); // what rooms sent during that turn is the next turn's, and in this session it starts now
            }
            "control_request" if msg["request"]["subtype"] == "can_use_tool" => {
                // The rules already ran in the PreToolUse hook; anything that reaches here is allowed.
                let _ = tx.send(control_reply(&msg, json!({"behavior": "allow", "updatedInput": msg["request"]["input"]})));
            }
            _ => {}
        }
    }
    // What a dead turn left is never sent to a room with a later answer; what rooms wrote for it is handed over again, once, and before the wait below, so a line that arrives during it (and waits in `pending`) is not handed twice.
    let died = talk(project, |t| (t.tx.take().is_some() && t.busy || !t.turn_rooms.is_empty()) && ready); // a session that never got ready ran no turn: its lines wait, with their rooms; a promoted turn it never answered counts as died too
    let _ = died.then(|| (talk(project, |t| (t.reply_to.clear(), t.turn_rooms.clear())), rooms::again(app, Some(project))));
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
fn history(app: &Shared, project: &str) -> String {
    let rows: Vec<String> = app.query("SELECT role, text FROM chat WHERE project = ?1 ORDER BY id DESC LIMIT 30", [project], |r| Ok(format!("{}: {}", r.get::<_, String>(0)?, r.get::<_, String>(1)?)));
    format!("Recent conversation for context (do not repeat completed work):\n{}\n\nNew messages:\n", rows.into_iter().rev().collect::<Vec<_>>().join("\n"))
}
async fn exec_session(app: &Shared, project: &str, muse: bool) -> Result<(), String> {
    let key = format!("manager_session:{}:{project}", if muse { "muse" } else { "codex" });
    let resumed = app.kv(&key);
    // The turn's message in plain text, with the recent conversation in front of it when the agent starts fresh; `sent` is handed over again when the turn never started.
    let sent = talk(project, |t| std::mem::take(&mut t.pending));
    begin_turn(app, project); // this exec turn received exactly that waiting input; what arrives while it runs waits for the next one
    let mut input = if resumed.is_none() { history(app, project) } else { String::new() };
    input += &sent.iter().map(|s| format!("{s}\n")).collect::<String>();
    let mut started = false;
    let result = crate::codex::turn(
        app,
        &app.chat_role(project),
        project,
        resumed.as_deref(),
        (&prompt(app, project), &input),
        |msg| {
            if muse {
                msg["stream"]["id"].as_str().filter(|_| msg["stream"]["kind"] == "session").into_iter().for_each(|id| app.set_kv(&key, Some(id)));
                if msg["payload_type"] == "tool.result" {
                    // The shell's own result text is its result JSON: the command it ran is the line worth showing.
                    let detail = msg["payload"]["text"].as_str().and_then(|s| serde_json::from_str::<Value>(s).ok()).and_then(|v| v["command"].as_str().map(String::from));
                    app.chat("tool", detail.as_deref().unwrap_or_else(|| msg["payload"]["correlation_facts"]["tool_name"].as_str().unwrap_or("tool")), project);
                    typing_line(project, msg["payload"]["correlation_facts"]["tool_name"].as_str().unwrap_or("tool"));
                    // the phones get the tool's name, never its raw command
                }
                return;
            }
            match msg["type"].as_str().unwrap_or("") {
                "thread.started" => msg["thread_id"].as_str().into_iter().for_each(|id| app.set_kv(&key, Some(id))),
                "item.completed" if msg["item"]["type"] == "agent_message" => {
                    let text = msg["item"]["text"].as_str().unwrap_or("");
                    app.chat("manager", text, project);
                    let _ = talk(project, |t| t.phones).then(|| rooms::to_phones(app, project, text));
                }
                "item.started" if msg["item"]["type"] == "command_execution" => {
                    let cmd = msg["item"]["command"].as_str().unwrap_or("command");
                    app.chat("tool", cmd, project);
                    typing_line(project, cmd.split_whitespace().find(|w| !w.contains('=')).unwrap_or("command"));
                    // only the program's name: its arguments may hold anything
                }
                _ => {}
            }
        },
        |pid| {
            started = true;
            talk(project, |t| ((t.pid, t.stale) = (pid, false), set_busy(app, project, t, true)));
            typing_start(app, project, false);
        },
    )
    .await;
    if !started {
        talk(project, |t| (t.reply_to.append(&mut t.turn_rooms), drop(t.pending.splice(0..0, sent)))); // no turn ran: its rooms wait again with its input
        return result.map(|_| ());
    }
    let (rooms_now, phones) = talk(project, |t| (set_busy(app, project, t, false), (std::mem::take(&mut t.turn_rooms), std::mem::take(&mut t.phones))).1);
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
    typing_stop(project); // finished, failed or timed out: the phones stop showing it typing
    rooms::reply(app, &Vec::from_iter(rooms_now), &text);
    Ok(())
}
#[cfg(test)]
#[path = "../checks/manager.rs"]
mod tests;
