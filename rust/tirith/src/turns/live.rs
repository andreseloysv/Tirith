//! The turns of the conversations as they run: the trigger waiting for the next message, the turn open now, and the messages handed during it, each with its own trigger.
use super::record::Turn;
use super::save::finish;
use crate::app::Shared;
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;

/// What started a turn: its trigger and its routine's id (empty when none).
pub type By = (String, String);

/// One conversation's turns: the trigger waiting for the message that is composed next, the turn open now, and the messages handed to its session during it (each starts its own turn, in order).
#[derive(Default)]
pub(super) struct Conv {
    trigger: Option<By>,
    pub(super) open: Option<Turn>,
    queued: VecDeque<By>,
}
static CONVS: Mutex<BTreeMap<String, Conv>> = Mutex::new(BTreeMap::new());
pub(super) fn with<T>(conv: &str, f: impl FnOnce(&mut Conv) -> T) -> T {
    f(CONVS.lock().unwrap().entry(conv.to_string()).or_default())
}

/// What starts the turn of the next message `conv` is handed: `owner`, `room`, `schedule`, `run-now` or `event:<name>`, with the routine's id. The first one counts, and whoever composes the next message takes it (`take_trigger`), so it never outlives that message.
pub fn trigger(conv: &str, trigger: &str, routine: &str) {
    with(conv, |c| {
        c.trigger
            .get_or_insert_with(|| (trigger.into(), routine.into()));
    });
}
/// An event handed to `conv` as a turn starts its next turn (a note waits for another message: it starts none).
pub fn on_event(conv: &str, on: &str, mode: &str) {
    let Some(()) = (mode == "turn").then_some(()) else {
        return;
    };
    let by = if on == "schedule" {
        on.to_string()
    } else {
        format!("event:{on}")
    };
    trigger(conv, &by, "");
}
/// The trigger waiting under `key` (empty when nobody set one), taken.
pub fn take_trigger(key: &str) -> By {
    with(key, |c| c.trigger.take()).unwrap_or_default()
}

/// A message went to the session of `conv`: it starts a turn by `by`, or, during one, its own turn starts at that one's end.
pub fn handed(app: &Shared, conv: &str, by: By) {
    let turn = with(conv, |c| c.open.is_none()).then(|| {
        let t = Turn::new(app, conv, &app.chat_role(conv), by.clone());
        super::interrupted::mark(app, &t);
        t
    });
    with(conv, |c| match turn.filter(|_| c.open.is_none()) {
        Some(t) => c.open = Some(t),
        None => c.queued.push_back(by),
    });
}
/// True when no turn of `conv` is open or waiting to start.
pub fn idle(conv: &str) -> bool {
    with(conv, |c| c.open.is_none() && c.queued.is_empty())
}
/// A line of the agent of `conv` during its turn.
pub fn line(conv: &str, msg: &Value) {
    with(conv, |c| c.open.iter_mut().for_each(|t| t.line(msg)));
}
/// The turn of `conv` ended (`ok` and `answer` when its caller knows them): its row is kept, and the next message handed during it starts its turn.
pub fn end(app: &Shared, conv: &str, ok: Option<bool>, answer: Option<&str>) {
    let (open, next) = with(conv, |c| (c.open.take(), c.queued.pop_front()));
    open.into_iter().for_each(|t| {
        finish(app, t, ok, answer);
    });
    next.into_iter().for_each(|by| handed(app, conv, by));
}
/// The session of `conv` ended: a turn it left open failed, unless nothing came during it, and what its run held (a seat in a live experiment, a tuning of its routine) is freed.
pub fn abandon(app: &Shared, conv: &str) {
    let (open, queued) = with(conv, |c| (c.open.take(), std::mem::take(&mut c.queued)));
    let used = open.is_some() || !queued.is_empty();
    open.filter(|t| t.seen).into_iter().for_each(|t| {
        finish(app, t, Some(false), None);
    });
    let _ = used.then(|| crate::manager::release_run(app, conv));
}
