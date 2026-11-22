//! The owner's routes of a sign-in (refused to every agent here too, besides `server/scope.rs`): begin one, ask how it stands. Neither answers a token; the callback is the browser's.
use super::{callback, pending, start::start};
use crate::app::{bad, Shared, R};
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::middleware::{from_fn, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

/// Whether `path` is one of the owner's sign-in routes.
pub fn route(path: &str) -> bool {
    path.starts_with("/api/connectors/")
}

pub fn routes() -> Router<Shared> {
    let owners = Router::new()
        .route("/api/connectors/{id}/login", post(begun))
        .route("/api/connectors/logins/{ticket}", get(stands))
        .route_layer(from_fn(owner_only));
    owners.route(callback::PATH, get(callback::callback))
}

async fn owner_only(req: Request, next: Next) -> Response {
    let who = req.extensions().get::<crate::server::Who>();
    if !matches!(who, Some(crate::server::Who::Owner)) {
        let why = "a connector's sign-in is the owner's alone";
        return (StatusCode::FORBIDDEN, why).into_response();
    }
    next.run(req).await
}

async fn begun(State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>) -> R {
    let bot = b["bot"].as_str().unwrap_or_default();
    let (url, ticket) = start(&app, &id, bot).await.map_err(bad)?;
    Ok(Json(json!({"url": url, "ticket": ticket})))
}

async fn stands(Path(ticket): Path<String>) -> R {
    let said = pending::status(&ticket);
    said.map(Json)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "no such sign-in".into()))
}
