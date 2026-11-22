//! The turns of the conversations as they run: who started the next one, the one open now, and its end.
use super::record::Turn;
use super::save::finish;
use crate::app::Shared;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Mutex;

/// One conversation's turns: the trigger waiting for its next turn, the turn open now, and whether more was handed to it while it ran.
#[derive(Default)]
pub(super) struct Conv {
    trigger: Option<(String, String)>,
    pub(super) open: Option<Turn>,
    more: bool,
}
static CONVS: Mutex<BTreeMap<String, Conv>> = Mutex::new(BTreeMap::new());
pub(super) fn with<T>(conv: &str, f: impl FnOnce(&mut Conv) -> T) -> T {
    f(CONVS.lock().unwrap().entry(conv.to_string()).or_default())
}

/// What started the next turn of `conv`: `owner`, `room`, `schedule`, `run-now` or `event:<name>`, with the routine's id. Whoever hands over the message sets it, and the first one counts until the turn starts.
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
pub fn take_trigger(key: &str) -> (String, String) {
    with(key, |c| c.trigger.take()).unwrap_or_default()
}

/// The turn of `conv` starts now (a session took its message): a turn already open stays the one running.
pub fn begin(app: &Shared, conv: &str) {
    let Some(()) = with(conv, |c| c.open.is_none()).then_some(()) else {
        return;
    };
    let turn = Turn::new(app, conv, &app.chat_role(conv), take_trigger(conv));
    with(conv, |c| (c.open.get_or_insert(turn), c.more = false).1);
}
/// A message went to the live session of `conv`: it starts a turn, or, during one, the next one starts at its end.
pub fn handed(app: &Shared, conv: &str) {
    let Some(()) = with(conv, |c| (c.more |= c.open.is_some(), c.open.is_none()).1).then_some(())
    else {
        return;
    };
    begin(app, conv);
}
/// A line of the agent of `conv` during its turn.
pub fn line(conv: &str, msg: &Value) {
    with(conv, |c| c.open.iter_mut().for_each(|t| t.line(msg)));
}
/// The turn of `conv` ended (`ok` and `answer` when its caller knows them): its row is kept, and what was handed during it starts the next.
pub fn end(app: &Shared, conv: &str, ok: Option<bool>, answer: Option<&str>) {
    let (open, more) = with(conv, |c| (c.open.take(), std::mem::take(&mut c.more)));
    open.into_iter().for_each(|t| {
        finish(app, t, ok, answer);
    });
    let _ = more.then(|| begin(app, conv));
}
/// The session of `conv` ended: a turn it left open failed, unless nothing came during it.
pub fn abandon(app: &Shared, conv: &str) {
    let open = with(conv, |c| (c.more = false, c.open.take()).1).filter(|t| t.seen);
    open.into_iter().for_each(|t| {
        finish(app, t, Some(false), None);
    });
}

/// One fresh turn of a turn role (`run_turn`), started by what waits under `turn:<role>` (an event, when nobody said).
pub fn turn_of(app: &Shared, role: &str, project: &str) -> Turn {
    let (trigger, routine) = take_trigger(&format!("turn:{role}"));
    let trigger = if trigger.is_empty() {
        "event".into()
    } else {
        trigger
    };
    let conv = if project.is_empty() { role } else { project };
    let mut turn = Turn::new(app, conv, role, (trigger, routine));
    turn.loose = true;
    turn
}
/// A turn role's turn ended with `result`: its row is kept, and the result goes back as it was.
pub fn done(app: &Shared, t: Turn, result: Result<String, String>) -> Result<String, String> {
    let answer = result.clone().unwrap_or_else(|e| e);
    finish(app, t, Some(result.is_ok()), Some(&answer));
    result
}
