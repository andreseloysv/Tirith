//! The teach API: the recording read, started and stopped (by the page, or by the extension with the token).
use super::cors::{allow, from_extension, has_token, preflight, refused};
use super::record::{start_now, stop_now};
use super::state::{left_ms, live, read, token};
use crate::app::Shared;
use crate::server::Who;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
/// The bot's conversation, if `id` is a bot and the caller may act for it: the page, or the extension with its token (`always`: the token is needed from anyone; the gate lets no agent be the extension).
pub(super) fn key(
    app: &Shared,
    h: &HeaderMap,
    id: &str,
    always: bool,
) -> Result<String, Box<Response>> {
    let key = crate::bots::key_for(app, id)
        .ok_or_else(|| allow(h, (StatusCode::NOT_FOUND, format!("no bot '{id}'"))))?;
    let needs = always || from_extension(h);
    (!needs || has_token(h, &token(app, &key)))
        .then_some(key)
        .ok_or_else(|| Box::new(allow(h, refused())))
}
async fn show(
    State(app): State<Shared>,
    who: Who,
    h: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let key = match key(&app, &h, &id, false) {
        Ok(k) => k,
        Err(r) => return *r,
    };
    let mut rec = read(&app, &key);
    if rec["on"] == true && !live(&rec) {
        stop_now(&app, &key, ""); // its 10 minutes ran out while nobody was looking: handed over now, once
        rec = read(&app, &key);
    }
    rec["on"] = json!(live(&rec));
    rec["left_ms"] = json!(left_ms(&rec));
    rec["rehearsals"] = json!(super::rehearsal::shown(&app, &key));
    let page = !from_extension(&h) && matches!(who, Who::Owner); // the token makes its holder the extension
    let _ = page.then(|| rec["token"] = json!(token(&app, &key)));
    allow(&h, Json(rec))
}
async fn start(State(app): State<Shared>, h: HeaderMap, Path(id): Path<String>) -> Response {
    match key(&app, &h, &id, false) {
        Ok(k) => allow(&h, Json(start_now(&app, &k))),
        Err(r) => *r,
    }
}
async fn stop(State(app): State<Shared>, h: HeaderMap, Path(id): Path<String>) -> Response {
    match key(&app, &h, &id, false) {
        Ok(k) => allow(&h, Json(json!({"ok": stop_now(&app, &k, ""), "on": false}))),
        Err(r) => *r,
    }
}
pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/teach/{id}", get(show).options(preflight))
        .route("/api/teach/{id}/start", post(start).options(preflight))
        .route("/api/teach/{id}/stop", post(stop).options(preflight))
        .route(
            "/api/teach/{id}/events",
            post(super::events::events).options(preflight),
        )
        .route("/api/teach/{id}/approve", post(super::approve::approve))
        .merge(super::rehearse_api::routes())
}
