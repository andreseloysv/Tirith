//! The owner's routes, which refuse whoever is not the owner themselves too (besides `server/scope.rs`): a value comes in, and only names go out.
use super::{names, store::Store};
use crate::app::{bad, Shared, R};
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::middleware::{from_fn, Next};
use axum::response::{IntoResponse, Response};
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
        .route_layer(from_fn(owner_only))
}

/// Here as well as in the server's list: a list that lost these routes would give them to every agent.
async fn owner_only(req: Request, next: Next) -> Response {
    let who = req.extensions().get::<crate::server::Who>();
    if !matches!(who, Some(crate::server::Who::Owner)) {
        return (
            StatusCode::FORBIDDEN,
            "a bot's secrets are the owner's alone",
        )
            .into_response();
    }
    next.run(req).await
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
        names::owners(field("name"))?;
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
