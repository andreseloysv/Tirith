//! The owner's routes of the Connectors page (refused to every agent here too, besides `server/scope.rs`, which keeps `/api/connectors/` the owner's): the page's state and Remove. Login is `oauth/`'s route, Block and Unblock `access/`'s.
use super::{remove::remove, state::page};
use crate::app::{bad, Shared, R};
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::middleware::{from_fn, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/connectors/page", get(shown))
        .route("/api/connectors/{id}/remove", post(removed))
        .route_layer(from_fn(owner_only))
}

/// What only the owner reaches: refused to any agent's token.
pub async fn owner_only(req: Request, next: Next) -> Response {
    let who = req.extensions().get::<crate::server::Who>();
    if !matches!(who, Some(crate::server::Who::Owner)) {
        let why = "the connectors and their grants are the owner's alone";
        return (StatusCode::FORBIDDEN, why).into_response();
    }
    next.run(req).await
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| "the work stopped".to_string())?
}

async fn shown(State(app): State<Shared>) -> R {
    Ok(Json(blocking(move || Ok(page(&app))).await.map_err(bad)?))
}

async fn removed(State(app): State<Shared>, Path(id): Path<String>) -> R {
    let said = blocking(move || remove(&app, &id)).await.map_err(bad)?;
    Ok(Json(json!({"said": said})))
}
