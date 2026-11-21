//! The teach API: the recording read, started and stopped (by the page, or by the extension with the token), and the extension's steps.
use super::cors::{allow, from_extension, has_token, preflight, refused};
use super::mask::kept;
use super::record::{start_now, stop_now};
use super::state::{live, now_ms, push, read, token};
use crate::app::Shared;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
/// The bot's conversation, if `id` is a bot and the caller may act for it: the page, or the extension with its token (`always`: the token is needed from anyone).
fn key(app: &Shared, h: &HeaderMap, id: &str, always: bool) -> Result<String, Box<Response>> {
    let key = crate::bots::key_for(app, id)
        .ok_or_else(|| allow(h, (StatusCode::NOT_FOUND, format!("no bot '{id}'"))))?;
    let needs = always || from_extension(h);
    (!needs || has_token(h, &token(app, &key)))
        .then_some(key)
        .ok_or_else(|| Box::new(allow(h, refused())))
}
async fn show(State(app): State<Shared>, h: HeaderMap, Path(id): Path<String>) -> Response {
    let key = match key(&app, &h, &id, false) {
        Ok(k) => k,
        Err(r) => return *r,
    };
    let mut rec = read(&app, &key);
    rec["on"] = json!(live(&rec));
    let _ = (!from_extension(&h)).then(|| rec["token"] = json!(token(&app, &key)));
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
/// The extension's steps: one, or `{"steps": [...]}`; kept only while recording, each masked again here.
async fn events(
    State(app): State<Shared>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    let key = match key(&app, &h, &id, true) {
        Ok(k) => k,
        Err(r) => return *r,
    };
    let rec = read(&app, &key);
    if rec["on"] == true && !live(&rec) {
        stop_now(&app, &key, ""); // its 10 minutes are over
    }
    let b: Value = serde_json::from_slice(&body).unwrap_or_default();
    let steps = b["steps"].as_array().cloned().unwrap_or_else(|| vec![b]);
    let now = now_ms();
    let took = steps
        .iter()
        .filter_map(|st| kept(st, &now))
        .filter(|st| push(&app, &key, st.clone()))
        .count();
    let _ = (took > 0).then(|| app.emit("teach", json!({"bot": key})));
    let on = live(&read(&app, &key));
    allow(&h, Json(json!({"ok": true, "kept": took, "on": on})))
}
pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/teach/{id}", get(show).options(preflight))
        .route("/api/teach/{id}/start", post(start).options(preflight))
        .route("/api/teach/{id}/stop", post(stop).options(preflight))
        .route("/api/teach/{id}/events", post(events).options(preflight))
}
