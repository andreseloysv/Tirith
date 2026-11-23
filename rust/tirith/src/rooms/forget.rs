//! The owner has OpenBot forget a phone.
use super::{store::store, store::ROOMS, Peer, Room, PHONE};
use crate::app::{bad, Shared, R};
use axum::{extract::State, Json};
use serde_json::{json, Value};
/// Forgets a phone (by its key id, or 8 or more of its first characters) in every room: what it writes is no longer read and nothing more is sealed for it, until it is paired again with a new QR.
pub(super) async fn api_forget(State(app): State<Shared>, Json(b): Json<Value>) -> R {
    let (key, mut found) = (b["key_id"].as_str().unwrap_or("").to_string(), 0);
    // Every room in one go, under the rooms' lock: nothing puts the phone back half way.
    for r in ROOMS.lock().unwrap().values_mut() {
        if forget_in(r, &key) {
            store(&app, r);
            found += 1;
        }
    }
    (found > 0)
        .then(|| Json(json!({"ok": true, "rooms": found})))
        .ok_or(bad(format!("no phone {key}")))
}
/// True when the phone was in the room: it is one of its blocked managers now, and what it wrote is no longer owed.
fn forget_in(r: &mut Room, key: &str) -> bool {
    let (gone, kept): (Vec<Peer>, Vec<Peer>) = std::mem::take(&mut r.devices)
        .into_iter()
        .partition(|d| key.len() >= 8 && d.key_id.starts_with(key));
    r.devices = kept;
    let found = !gone.is_empty();
    if found {
        r.owes
            .owed
            .retain(|o| !gone.iter().any(|d| o.0 == format!("{} {PHONE}", d.name)));
        r.managers.extend(gone.into_iter().map(|p| Peer {
            accepted: false,
            blocked: true,
            ..p
        }));
    }
    found
}
