//! The extension's side of a rehearsal, with the bot's token: the long poll for the next step, and its answer with a result and a screenshot.
use super::api::key;
use super::cors::allow;
use super::rehearsal::{end, last, live, until_s};
use super::relay;
use crate::app::Shared;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::Json;
use serde_json::{json, Value};
use std::time::Duration;
/// How long the extension's long poll for the next step waits.
const POLL_S: u64 = 20;
/// The extension's long poll: the next step, or none after a while; `on` false once the rehearsal is over.
pub(super) async fn next(
    State(app): State<Shared>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let key = match key(&app, &h, &id, true) {
        Ok(k) => k,
        Err(r) => return *r,
    };
    let r = last(&app, &key).unwrap_or_default();
    if !live(&r) {
        let _ = (r["on"] == true).then(|| end(&app, &key));
        return allow(&h, Json(json!({"on": false})));
    }
    let cmd = relay::next(&key, Duration::from_secs(POLL_S)).await;
    allow(
        &h,
        Json(json!({"on": true, "until": until_s(&r) * 1000, "cmd": cmd})),
    )
}
/// The extension's answer to a step: `{id, ok, result, shot}` (the screenshot as a data URL).
pub(super) async fn done(
    State(app): State<Shared>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    let key = match key(&app, &h, &id, true) {
        Ok(k) => k,
        Err(r) => return *r,
    };
    let b: Value = serde_json::from_slice(&body).unwrap_or_default();
    let took = relay::answer(&key, b["id"].as_u64().unwrap_or(0), b);
    allow(&h, Json(json!({"ok": took})))
}
