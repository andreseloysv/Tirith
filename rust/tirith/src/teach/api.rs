//! The teach API: the recording read, started and stopped (by the page, or by the extension with the token), and the extension's steps.
use super::cors::{allow, from_extension, has_token, preflight, refused};
use super::hand::over;
use super::mask::kept;
use super::state::{change, live, now_ms, push, read, token, MOST_S};
use crate::app::{now_iso, now_s, Shared};
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
/// Stops the recording `since` (any, when empty) and hands it over; false when none was on.
fn stop_now(app: &Shared, key: &str, since: &str) -> bool {
    let rec = change(app, key, |rec| {
        let on = rec["on"] == true && (since.is_empty() || rec["since"] == since);
        rec["on"] = json!(false);
        on.then(|| rec.clone())
    });
    let _ = rec.as_ref().map(|r| over(app, key, r));
    app.emit("teach", json!({"bot": key}));
    rec.is_some()
}
fn start_now(app: &Shared, key: &str) -> Value {
    let since = now_ms();
    change(
        app,
        key,
        |rec| *rec = json!({"on": true, "since": since, "since_s": now_s(), "steps": []}),
    );
    app.chat("tirith", "Recording in Edge: do the task once; what you write here meanwhile goes with the steps. It stops by itself in 10 minutes.", key);
    app.emit("teach", json!({"bot": key}));
    let (app, key2) = (app.clone(), key.to_string());
    let since2 = since.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(MOST_S)).await;
        stop_now(&app, &key2, &since2);
    });
    json!({"ok": true, "on": true, "since": since, "at": now_iso()})
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
