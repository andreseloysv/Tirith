//! A routine's run has its agent's turn to itself: what comes meanwhile waits, and a live session is handed it when that turn ends (the streaming CLI would fold it into the running one).
use super::deliver::user_line;
use super::jobs::take_group;
use super::talk::{set_busy, Msg, Talk};
use crate::app::Shared;

/// True while a routine's run has the conversation.
pub(super) fn held(t: &Talk) -> bool {
    t.taken || t.in_run
}

/// The turn of `group` starts: whether it is a routine's run, and whether its answer reaches the phones.
pub(super) fn starts(t: &mut Talk, group: &[Msg]) {
    t.in_run = group.first().is_some_and(|m| m.job);
    t.phones |= group.iter().any(|m| m.phones);
}

/// The live session's turn ended: what waited for it is sent now, a routine's run alone.
pub(super) fn hand_waiting(app: &Shared, project: &str, t: &mut Talk) -> bool {
    let Some(tx) =
        t.tx.clone()
            .filter(|tx| !t.pending.is_empty() && !tx.is_closed())
    else {
        return false;
    };
    let group = take_group(&mut t.pending);
    starts(t, &group);
    for m in group {
        let _ = tx.send(user_line(&m.text));
        crate::turns::handed(app, project, m.by);
    }
    set_busy(app, project, t, true);
    true
}
