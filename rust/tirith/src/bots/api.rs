//! The bots' HTTP API: the list and its cards, a bot deleted, its routines, a routine run now, its Library.
use super::card::save;
use super::folder::folder;
use super::library::{learn, library};
use super::list::{all, bot, key_for};
use super::routine::routine;
use super::run::fire;
use super::store::with;
use crate::app::{bad, Shared, R};
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
fn reply(res: Result<Value, String>) -> R {
    res.map(Json).map_err(bad)
}
/// An owner's bot goes, with its room, its memory and its own skills; a system bot stays.
async fn delete(State(app): State<Shared>, Path(id): Path<String>) -> R {
    let key = key_for(&app, &id)
        .filter(|k| k.starts_with("bot-"))
        .ok_or_else(|| bad(format!("no bot '{id}' (a system bot stays)")))?;
    with(&app, |bots| {
        bots.iter()
            .position(|b| b["id"] == id.as_str())
            .map(|i| bots.remove(i))
            .ok_or(format!("no bot '{id}'"))
    })
    .map_err(bad)?;
    crate::rooms::forget(&app, &key);
    app.set_kv(&format!("webhook:{id}"), None); // its room is gone, and with it what the webhook posts
    let _ = std::fs::remove_dir_all(folder(&app, &key)); // its memory and its own skills
    reply(Ok(json!({"ok": true})))
}
async fn list(State(app): State<Shared>, Path(id): Path<String>) -> R {
    reply(
        key_for(&app, &id)
            .and_then(|k| bot(&app, &k))
            .map(|b| b["routines"].clone())
            .ok_or(format!("no bot '{id}'")),
    )
}
async fn change(State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>) -> R {
    reply(routine(&app, id, b))
}
async fn run(State(app): State<Shared>, Path((id, rid)): Path<(String, String)>) -> R {
    let key = key_for(&app, &id).unwrap_or_default();
    crate::turns::trigger(&key, "run-now", &rid);
    reply(fire(&app, &id, &rid, "").map(|_| json!({"ok": true})))
}
async fn lib(State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>) -> R {
    reply(learn(&app, &id, &b))
}
pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/bots", get(|State(app): State<Shared>| async move { Json(json!(all(&app))) }).post(|State(app): State<Shared>, Json(b): Json<Value>| async move { reply(save(&app, b)) }))
        .route("/api/bots/{id}/delete", post(delete))
        .route("/api/bots/{id}/routines", get(list).post(change))
        .route("/api/bots/{id}/run/{routine}", post(run))
        .merge(crate::turns::routes())
        .merge(super::hook_api::routes())
        .merge(super::from_template::routes())
        .route("/api/bots/{id}/library", get(|State(app): State<Shared>, Path(id): Path<String>| async move { reply(library(&app, &id)) }).post(lib))
}
