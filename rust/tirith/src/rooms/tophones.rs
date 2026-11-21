//! What a project's conversation says reaches the phones in its rooms.
use super::{outbox::queue, store::ROOMS, Room};
use crate::app::Shared;
/// The ids of a conversation's own rooms, the ones its phones are in.
pub fn phone_rooms(app: &Shared, project: &str) -> Vec<String> {
    ROOMS
        .lock()
        .unwrap()
        .values()
        .filter(|r| r.kind == "manager" && app.conv(&r.project) == project)
        .map(|r| r.id.clone())
        .collect()
}
/// What a project's conversation says (its manager's words, Tirith's reports, a card) goes to the phones of its rooms.
pub fn to_phones(app: &Shared, project: &str, text: &str) {
    // An answer that starts with ∅ (a routine found nothing new) stays on the page: only news reaches the phones.
    let rooms: Vec<Room> = ROOMS
        .lock()
        .unwrap()
        .values()
        .filter(|r| {
            r.kind == "manager"
                && app.conv(&r.project) == project
                && !text.trim_start().starts_with('∅')
        })
        .cloned()
        .collect();
    rooms.iter().for_each(|r| {
        queue(
            &r.id,
            text,
            r.devices.iter().map(|d| d.public.clone()).collect(),
        )
    });
}
