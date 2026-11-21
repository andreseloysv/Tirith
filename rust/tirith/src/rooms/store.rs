//! The rooms in memory and on disk: one lock for all of them, one row per room, and the page told of each change.
use super::{keeper::WAKE, view::view, Room};
use crate::app::Shared;
use axum::http::StatusCode;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Mutex;
pub(super) static ROOMS: Mutex<BTreeMap<String, Room>> = Mutex::new(BTreeMap::new());
pub(super) fn room(id: &str) -> Option<Room> {
    ROOMS.lock().unwrap().get(id).cloned()
}
pub(super) fn known(id: &str) -> Result<Room, (StatusCode, String)> {
    room(id).ok_or((StatusCode::NOT_FOUND, format!("no room {id}")))
}
pub(super) fn store(app: &Shared, r: &Room) {
    let data = serde_json::to_string(r).unwrap_or_default();
    let _ = app.db.lock().unwrap().execute(
        "INSERT OR REPLACE INTO am_rooms (id, data) VALUES (?1, ?2)",
        [&r.id, &data],
    );
    app.emit("room", view(app, r));
}
pub(super) fn save(app: &Shared, r: &Room) {
    ROOMS.lock().unwrap().insert(r.id.clone(), r.clone());
    store(app, r);
}
/// Changes a record in place under the rooms' lock, so two tasks never undo each other's change (`f` must not read or change the rooms itself). `None` when the room is gone (left meanwhile). Written only when what is kept changed.
pub(super) fn update<T>(app: &Shared, id: &str, f: impl FnOnce(&mut Room) -> T) -> Option<T> {
    let mut rooms = ROOMS.lock().unwrap();
    let r = rooms.get_mut(id)?;
    let before = serde_json::to_string(r).unwrap_or_default();
    let out = f(r);
    let _ = (serde_json::to_string(r).unwrap_or_default() != before).then(|| store(app, r));
    Some(out)
}
/// The rooms kept at the last stop come back.
pub(super) fn load(app: &Shared) {
    let rows: Vec<String> = app.query("SELECT data FROM am_rooms", [], |r| r.get(0));
    let mut loaded: Vec<Room> = rows
        .iter()
        .filter_map(|s| serde_json::from_str::<Room>(s).ok())
        .collect();
    loaded
        .iter_mut()
        .filter(|r| r.kind == "manager" && r.project.is_empty())
        .for_each(|r| (r.project = "manager-general".into(), store(app, r)).1); // the room of no project is the manager-general bot's now, and is kept that way
    ROOMS
        .lock()
        .unwrap()
        .extend(loaded.into_iter().map(|r| (r.id.clone(), r)));
}
pub(super) fn drop_rooms(app: &Shared, gone: impl Fn(&Room) -> bool) {
    ROOMS
        .lock()
        .unwrap()
        .values()
        .filter(|r| gone(r))
        .map(|r| r.id.clone())
        .collect::<Vec<_>>()
        .iter()
        .for_each(|id| {
            ROOMS.lock().unwrap().remove(id);
            app.insert("DELETE FROM am_rooms WHERE id = ?1", rusqlite::params![id]);
            app.emit("room", json!({"id": id, "gone": true}));
        });
    WAKE.notify_one(); // the phones are told that it is gone
}
