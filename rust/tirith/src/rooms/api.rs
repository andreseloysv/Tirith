//! The page's changes to a room: a new QR, a code for another manager, leaving it.
use super::store::{drop_rooms, known, update};
use super::{pair::new_code, Room};
use crate::app::{bad, now_s, Shared, R};
use axum::extract::{Path, State};
use axum::{http::StatusCode, Json};
use serde_json::json;
/// Changes a room this manager made (pairing phones, inviting managers): 400 `refusal` for any other room, 404 when it is gone.
fn as_manager<T>(
    app: &Shared,
    id: &str,
    refusal: &str,
    f: impl FnOnce(&mut Room) -> T,
) -> Result<T, (StatusCode, String)> {
    update(app, id, |r| (r.kind == "manager").then(|| f(r)))
        .ok_or((StatusCode::NOT_FOUND, format!("no room {id}")))?
        .ok_or_else(|| bad(refusal.into()))
}
pub(super) async fn api_qr(State(app): State<Shared>, Path(id): Path<String>) -> R {
    as_manager(
        &app,
        &id,
        "only the manager who made the room pairs phones",
        |r| (r.codes.invite, r.codes.invite_at) = (new_code(), now_s()),
    )?;
    Ok(Json(super::view::view(&app, &known(&id)?)))
}
pub(super) async fn api_invite(State(app): State<Shared>, Path(id): Path<String>) -> R {
    as_manager(
        &app,
        &id,
        "only the manager who made the room invites managers",
        |r| r.codes.peer_code = format!("{}:{}:{}", r.id, r.me_id, new_code()),
    )?;
    Ok(Json(json!({"peer_code": known(&id)?.codes.peer_code})))
}
pub(super) async fn api_leave(State(app): State<Shared>, Path(id): Path<String>) -> R {
    known(&id)?
        .private()
        .then_some(())
        .ok_or_else(|| bad("a public room is listed in config.toml: remove it there".into()))?;
    drop_rooms(&app, |r| r.id == id);
    Ok(Json(json!({"ok": true})))
}
/// The rooms of a conversation that is gone (a deleted bot) go too.
pub fn forget(app: &Shared, project: &str) {
    drop_rooms(app, |r| r.kind == "manager" && r.project == project);
}
