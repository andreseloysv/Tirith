//! One conversation's state, kept under its project's id (or "" for the one of no project), and what lists or ends its sessions.
use crate::app::{kill_group, Shared};
use crate::rooms;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Mutex;
use tokio::sync::mpsc;
/// One message waiting for the session on its way: its text, what starts its turn, and whether it is a routine's run (which a turn takes alone).
pub(super) struct Msg {
    pub(super) text: String,
    pub(super) by: crate::turns::By,
    pub(super) job: bool,
}
/// One run of a routine waiting for its conversation to be free: its task, what starts its turn (trigger and routine) and whether its answer reaches the phones.
pub(super) struct Job {
    pub(super) text: String,
    pub(super) by: crate::turns::By,
    pub(super) phones: bool,
}
/// One conversation: a project's (under its id) or the one of no project (under "").
#[derive(Default)]
pub(super) struct Talk {
    /// The live session's stdin. Without one, `pending` waits for the session that `alive` says is on its way.
    pub(super) tx: Option<mpsc::UnboundedSender<String>>,
    pub(super) pending: Vec<Msg>,
    pub(super) alive: bool,
    /// True while a turn runs (between a message handed to a live session and that session's `result`).
    pub(super) busy: bool,
    pub(super) pid: u32,
    /// Rooms whose messages wait for the next turn (`reply_to`, their answer goes back there), the rooms of the turn now running (`turn_rooms`: only what they were owed when it started stops being owed at its end) and the turn's text so far.
    pub(super) reply_to: BTreeSet<String>,
    pub(super) turn_rooms: BTreeSet<String>,
    pub(super) text: String,
    /// Reports from Tirith not yet handed over, and whether a hand-over is already on its way.
    pub(super) notes: Vec<String>,
    pub(super) due: bool,
    /// Reports cut from `notes` because it was full: said in the next message.
    pub(super) left_out: usize,
    /// The routines' runs, in the order they were due; each is handed over alone when the conversation is free.
    pub(super) jobs: VecDeque<Job>,
    /// The current (or next) turn was started by the owner or by an event whose route has `phone = true`: its answer reaches the phones.
    pub(super) phones: bool,
    /// The settings changed during a turn: the session ends when the turn does.
    pub(super) stale: bool,
    /// The current turn's live progress for the phones in its rooms (see `rooms::Typing`); idle when the turn has no rooms.
    pub(super) typing: rooms::Typing,
}
static TALKS: Mutex<BTreeMap<String, Talk>> = Mutex::new(BTreeMap::new());
pub(super) fn talk<T>(project: &str, f: impl FnOnce(&mut Talk) -> T) -> T {
    f(TALKS
        .lock()
        .unwrap()
        .entry(project.to_string())
        .or_default())
}
pub(super) fn set_busy(app: &Shared, project: &str, t: &mut Talk, on: bool) {
    t.busy = on;
    app.emit("busy", json!({"project": project, "on": on}));
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
    TALKS
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, t)| t.busy)
        .map(|(p, _)| p.clone())
        .collect()
}
