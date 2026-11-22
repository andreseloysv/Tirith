//! The owner's routes (`server/caller.rs` refuses every agent): a value comes in, and only names go out.
use super::store::Store;
use crate::app::{bad, Shared, R};
use axum::extract::{Path, State};
use axum::routing::{delete, get};
use axum::{Json, Router};
use serde_json::{json, Value};

/// Whether `path` is one of these routes, a bot's names included.
pub fn route(path: &str) -> bool {
    path.starts_with("/api/secrets/")
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/secrets/{bot}", get(list).post(set))
        .route("/api/secrets/{bot}/{name}", delete(remove))
}

/// The stores block, and the Keychain may wait for the owner: never on an async thread.
async fn blocking(work: impl FnOnce() -> Result<Value, String> + Send + 'static) -> R {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| bad("the secret store stopped".into()))?
        .map(Json)
        .map_err(bad)
}

async fn list(State(app): State<Shared>, Path(bot): Path<String>) -> R {
    blocking(move || Store::of(&app).list(&bot).map(|names| json!(names))).await
}

async fn set(State(app): State<Shared>, Path(bot): Path<String>, Json(b): Json<Value>) -> R {
    blocking(move || {
        let field = |key: &str| b[key].as_str().unwrap_or_default();
        Store::of(&app)
            .set(&bot, field("name"), field("value"))
            .map(|_| json!({"ok": true}))
    })
    .await
}

async fn remove(State(app): State<Shared>, Path((bot, name)): Path<(String, String)>) -> R {
    blocking(move || {
        Store::of(&app)
            .remove(&bot, &name)
            .map(|_| json!({"ok": true}))
    })
    .await
}
