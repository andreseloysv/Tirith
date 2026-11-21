//! Every room this manager made gets every paired phone, and the phones are told the list of rooms when it changed.
use super::{listing::list, outbox::queue, store::store, store::ROOMS, Peer, Room};
use crate::app::Shared;
/// In one go, under the rooms' lock: a phone forgotten meanwhile is not brought back by an older list.
pub(super) fn share(app: &Shared) {
    let mut rooms = ROOMS.lock().unwrap();
    let phones = phones(rooms.values());
    let list = list(app, rooms.values());
    let sig = format!(
        "{list}{}",
        phones.iter().map(|d| d.key_id.as_str()).collect::<String>()
    );
    for r in rooms.values_mut().filter(|r| r.kind == "manager") {
        add_phones(app, r, &phones);
        let _ = (std::mem::replace(&mut r.told.listed, sig.clone()) != sig).then(|| {
            queue(
                &r.id,
                &format!("tirith-rooms {list}"),
                r.devices.iter().map(|d| d.public.clone()).collect(),
            )
        });
    }
}
/// The phones paired in any of the manager's rooms, once each.
fn phones<'a>(rooms: impl Iterator<Item = &'a Room>) -> Vec<Peer> {
    let mut phones: Vec<Peer> = rooms
        .filter(|r| r.kind == "manager")
        .flat_map(|r| r.devices.clone())
        .collect();
    phones.sort_by(|a, b| a.key_id.cmp(&b.key_id));
    phones.dedup_by(|a, b| a.key_id == b.key_id);
    phones
}
/// A phone new to the room joins it, and is no longer one of its other managers.
fn add_phones(app: &Shared, r: &mut Room, phones: &[Peer]) {
    let new: Vec<Peer> = phones
        .iter()
        .filter(|p| !r.devices.iter().any(|d| d.key_id == p.key_id))
        .cloned()
        .collect();
    if !new.is_empty() {
        r.managers
            .retain(|m| !new.iter().any(|p| p.key_id == m.key_id));
        r.devices.extend(new);
        store(app, r);
    }
}
