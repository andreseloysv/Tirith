//! The manager's turns for rooms: what a turn received stops being owed only when it ends, and what no turn answered is handed again.
use super::{
    deliver::phone_command, outbox::queue, store::update, store::ROOMS, uncite::uncited, Room,
};
use crate::{app::Shared, manager};
/// A turn that `rooms` started begins: what they are owed now is what it received (`answering`, kept with the cursor), and only that stops being owed at its end; what arrives while it runs is a later turn's.
pub fn begin(app: &Shared, rooms: &[String]) {
    rooms.iter().for_each(|id| {
        update(app, id, |r| r.owes.answering = r.owes.owed.clone());
    });
}
/// A finishing turn's leaving of a room: what it received stops being owed (one copy each, so an identical line that arrived during it stays), and these readers get its answer.
fn answered(r: &mut Room) -> Vec<String> {
    let owes = &mut r.owes;
    std::mem::take(&mut owes.answering).iter().for_each(|a| {
        drop(
            owes.owed
                .iter()
                .position(|o| o == a)
                .map(|i| owes.owed.remove(i)),
        )
    });
    r.managers
        .iter()
        .filter(|m| m.accepted)
        .map(|m| m.public.clone())
        .chain(std::mem::take(&mut r.owes.turn))
        .collect()
}
/// The end of a turn that `rooms` started: the agents there (the accepted managers and whoever wrote for this turn) get the manager's answer, without its memory citations. The phones have it already (`to_phones`).
pub fn reply(app: &Shared, rooms: &[String], text: &str) {
    let text = uncited(text);
    rooms
        .iter()
        .for_each(|id| queue(id, &text, update(app, id, answered).unwrap_or_default()));
}
/// Hands the managers again, once, what rooms wrote and no finished turn answered: the session ended, or the daemon was restarted, in between. It stays owed until a turn that received it ends. `project` limits it to one conversation.
pub fn again(app: &Shared, project: Option<&str>) {
    let rooms: Vec<Room> = ROOMS
        .lock()
        .unwrap()
        .values()
        .filter(|r| !r.owes.owed.is_empty() && project.is_none_or(|p| app.conv(&r.project) == p))
        .cloned()
        .collect();
    rooms.iter().for_each(|r| hand_again(app, r));
}
fn hand_again(app: &Shared, r: &Room) {
    update(app, &r.id, |r| {
        (r.owes.answering.clear(), r.owes.owed.clone()).1
    })
    .unwrap_or_default()
    .iter()
    .filter(|(a, t)| !phone_command(app, r, a, t))
    .for_each(|(author, text)| {
        manager::from_room(
            app,
            &r.project,
            &r.id,
            &format!("{author}, handed to you again after a restart (do not repeat what is done)"),
            text,
        )
    });
}
