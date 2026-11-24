//! What a project's conversation says reaches the phones in its rooms.
use super::outbox::{queue, Line};
use super::{store::ROOMS, uncite::uncited, unsent, Room};
use crate::app::Shared;
/// The ids of a conversation's own rooms that a phone joined: with none of those, its notices go to the general conversation's.
pub fn phone_rooms(app: &Shared, project: &str) -> Vec<String> {
    ROOMS
        .lock()
        .unwrap()
        .values()
        .filter(|r| r.kind == "manager" && !r.devices.is_empty() && app.conv(&r.project) == project)
        .map(|r| r.id.clone())
        .collect()
}
/// What a project's conversation says (its manager's words, NeoBot's reports, a card) goes to the phones of its rooms, without a bot's memory citations.
pub fn to_phones(app: &Shared, project: &str, text: &str) {
    for (room, text, readers) in lines(app, project, text) {
        queue(&room, &text, readers);
    }
}
/// The same, for a line that must arrive: kept until it is sent.
pub fn to_phones_surely(app: &Shared, project: &str, text: &str) {
    for (room, text, readers) in lines(app, project, text) {
        unsent::keep(app, (room.clone(), text.clone(), readers.clone()));
        queue(&room, &text, readers);
    }
}
/// What is sent where.
pub(super) fn lines(app: &Shared, project: &str, text: &str) -> Vec<Line> {
    // A true ∅ (a routine looked and found nothing new) stays on the page: only news reaches the phones, and «∅ I could not» is news.
    if crate::expected::quiet(text) {
        return vec![];
    }
    let text = &crate::expected::honest(text);
    let text = uncited(text);
    let rooms: Vec<Room> = ROOMS
        .lock()
        .unwrap()
        .values()
        .filter(|r| r.kind == "manager" && !r.devices.is_empty() && app.conv(&r.project) == project)
        .cloned()
        .collect();
    let readers = |r: &Room| r.devices.iter().map(|d| d.public.clone()).collect();
    rooms
        .iter()
        .map(|r| (r.id.clone(), text.to_string(), readers(r)))
        .collect()
}
