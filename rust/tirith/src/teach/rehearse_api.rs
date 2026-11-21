//! The rehearsal API: the bot (through `tirith teach`) starts and stops a rehearsal and sends its steps; the extension, with the token, asks for the next step and answers it with a result and a screenshot.
use super::api::key;
use super::cors::{from_extension, preflight};
use super::rehearsal::{begin, end, last, live, until_s};
use super::{ext_api, run};
use crate::app::Shared;
use crate::server::s;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
/// The bot's conversation, for a call from the page or the bot's `tirith teach`, never from the extension.
pub(super) fn own(app: &Shared, h: &HeaderMap, id: &str) -> Result<String, Box<Response>> {
    match key(app, h, id, false) {
        Ok(_) if from_extension(h) => Err(Box::new(StatusCode::FORBIDDEN.into_response())),
        Ok(k) => Ok(k),
        Err(r) => Err(r),
    }
}
async fn rehearse(
    State(app): State<Shared>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(b): Json<Value>,
) -> Response {
    let key = match own(&app, &h, &id) {
        Ok(k) => k,
        Err(r) => return *r,
    };
    let r = begin(&app, &key, &s(&b, "skill"));
    let at = chrono::DateTime::from_timestamp(until_s(&r) as i64, 0).unwrap_or_default();
    let what = Some(s(&b, "skill"))
        .filter(|n| !n.is_empty())
        .map_or("a draft".into(), |n| format!("the draft «{n}»"));
    app.chat("tirith", &format!("Rehearsing {what} in your Edge, in its own tab. It stops before any step that needs your yes, and by itself at {} UTC.", at.format("%H:%M")), &key);
    app.emit("teach", json!({"bot": key}));
    Json(json!({"ok": true, "since": r["since"], "until": until_s(&r) * 1000})).into_response()
}
async fn stop(State(app): State<Shared>, h: HeaderMap, Path(id): Path<String>) -> Response {
    match own(&app, &h, &id) {
        Ok(key) => {
            let was = end(&app, &key);
            app.emit("teach", json!({"bot": key}));
            Json(json!({"ok": was})).into_response()
        }
        Err(r) => *r,
    }
}
/// One step of the rehearsal: `open` a URL, `click` a selector or text, `type` a value in a field; answered with the extension's result and the screenshot's path.
async fn step(
    State(app): State<Shared>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(b): Json<Value>,
) -> Response {
    let key = match own(&app, &h, &id) {
        Ok(k) => k,
        Err(r) => return *r,
    };
    let r = last(&app, &key).unwrap_or_default();
    let refuse = |code: StatusCode, e: &str| (code, Json(json!({"error": e}))).into_response();
    if r["on"] != true {
        return refuse(
            StatusCode::CONFLICT,
            "no rehearsal is on: start one with tirith teach rehearse \"<skill>\"",
        );
    }
    if !live(&r) {
        end(&app, &key);
        return refuse(StatusCode::GONE, "the rehearsal stopped by itself after 10 minutes: say in the chat where it stopped, or start a new one with tirith teach rehearse \"<skill>\"");
    }
    match run::step(&app, &key, &b, until_s(&r) * 1000).await {
        Ok(out) => Json(out).into_response(),
        Err((code, e)) => refuse(code, &e),
    }
}
pub(super) fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/teach/{id}/rehearse", post(rehearse))
        .route("/api/teach/{id}/rehearse/stop", post(stop))
        .route("/api/teach/{id}/do", post(step))
        .route(
            "/api/teach/{id}/next",
            get(ext_api::next).options(preflight),
        )
        .route(
            "/api/teach/{id}/done",
            post(ext_api::done)
                .options(preflight)
                .layer(DefaultBodyLimit::max(16 << 20)),
        )
        .route("/api/teach/{id}/shot/{file}", get(super::shots::shot))
}
