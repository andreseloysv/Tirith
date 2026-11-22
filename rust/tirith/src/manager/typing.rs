//! A turn's rooms and its typing on the phones: where it shows its progress, when it starts and stops.
use super::talk::{busy, talk};
use crate::app::{now_s, Shared};
use crate::rooms;
use std::collections::BTreeSet;
/// The rooms a starting turn shows its progress in: the rooms that started it, and, for a turn marked for the phones, its conversation's own rooms. A turn with neither (only the page) writes in no room.
pub(super) fn typing_rooms(app: &Shared, project: &str) -> Vec<String> {
    let (reply_to, phones) = talk(project, |t| {
        (
            t.reply_to
                .iter()
                .chain(t.turn_rooms.iter())
                .cloned()
                .collect::<BTreeSet<_>>(),
            t.phones,
        )
    });
    reply_to
        .into_iter()
        .chain(
            phones
                .then(|| rooms::phone_rooms(app, project))
                .into_iter()
                .flatten(),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
/// Shows a turn's typing in its rooms: at its start, and with `again` (another message for the running turn, whose phones may have been reopened) its current state once more, in every room of the turn by then.
pub(super) fn typing_start(app: &Shared, project: &str, again: bool) {
    let rooms = typing_rooms(app, project);
    rooms::typing(talk(project, |t| t.typing.start(rooms, now_s(), again)));
}
/// Every busy turn's heartbeat (called by the rooms loop, about every 30 s): due ones resend their current state to their rooms alone, the rest send nothing; a stopped turn has no rooms left, so its heartbeat is over with it.
pub fn typing_heartbeats() {
    // Sent under the talk's lock, so a heartbeat that was due as its turn ended can never land after that turn's typing off.
    busy()
        .iter()
        .for_each(|p| talk(p, |t| rooms::typing(t.typing.heartbeat(now_s()))));
}
pub(super) fn typing_line(project: &str, line: &str) {
    rooms::typing(talk(project, |t| t.typing.line(line, now_s())));
}
pub(super) fn typing_stop(project: &str) {
    rooms::typing(talk(project, |t| t.typing.stop()));
}
/// A turn starts (a live session took its message, a session took its waiting ones, or an exec turn took its input): its rooms are the ones waiting by then, and only what they were owed at this moment stops being owed when it ends. A routine's run has none: they wait with their message.
pub(super) fn begin_turn(app: &Shared, project: &str) {
    let rooms: Vec<String> = talk(project, |t| {
        (t.turn_rooms.is_empty() && !super::waiting::held(t))
            .then(|| {
                (
                    t.turn_rooms.append(&mut t.reply_to),
                    Vec::from_iter(t.turn_rooms.clone()),
                )
                    .1
            })
            .into_iter()
            .flatten()
            .collect()
    });
    rooms::begin(app, &rooms);
}
