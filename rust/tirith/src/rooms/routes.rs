//! The rooms' HTTP API for the page and the CLI.
use super::api::{api_invite, api_leave, api_qr};
use super::{create::create, forget::api_forget, join::join, pair::new_code, say::api_say};
use super::{store::known, store::ROOMS, svg::api_svg, view::seen, view::view};
use crate::app::{bad, Shared, R};
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/rooms", get(api_list).post(api_new))
        .route("/api/rooms/join", post(api_join))
        .route("/api/rooms/forget", post(api_forget))
        .route("/api/rooms/{id}", get(api_room))
        .route("/api/rooms/{id}/qr", post(api_qr))
        .route("/api/rooms/{id}/qr.svg", get(api_svg))
        .route("/api/rooms/{id}/invite", post(api_invite))
        .route("/api/rooms/{id}/leave", post(api_leave))
        .route("/api/rooms/{id}/say", post(api_say))
}
async fn api_list(State(app): State<Shared>, h: HeaderMap) -> Json<Value> {
    let rooms = ROOMS.lock().unwrap().values().cloned().collect::<Vec<_>>();
    Json(json!({"rooms": rooms.iter().map(|r| seen(&h, view(&app, r))).collect::<Vec<_>>()}))
}
async fn api_join(State(app): State<Shared>, Json(b): Json<Value>) -> R {
    join(
        &app,
        b["code"].as_str().unwrap_or(""),
        b["project"].as_str().unwrap_or(""),
    )
    .await
    .map(|r| Json(view(&app, &r)))
    .map_err(bad)
}
async fn api_room(State(app): State<Shared>, h: HeaderMap, Path(id): Path<String>) -> R {
    known(&id).map(|r| Json(seen(&h, view(&app, &r))))
}
/// A project has one room, where all the owner's phones are: asking for another gives that room a new one-use QR.
async fn api_new(State(app): State<Shared>, h: HeaderMap, Json(b): Json<Value>) -> R {
    made(app, b).await.map(|Json(v)| Json(seen(&h, v)))
}
async fn made(app: Shared, b: Value) -> R {
    let project = b["project"].as_str().unwrap_or("");
    let old = ROOMS
        .lock()
        .unwrap()
        .values()
        .find(|r| r.kind == "manager" && app.conv(&r.project) == app.conv(project))
        .map(|r| r.id.clone());
    let Some(id) = old else {
        return Ok(Json(view(
            &app,
            &create(&app, b["title"].as_str().unwrap_or(""), project, new_code())
                .await
                .map_err(bad)?,
        )));
    };
    api_qr(State(app), Path(id)).await
}
