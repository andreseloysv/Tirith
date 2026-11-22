//! The owner's routes of the connectors on the Access page (refused to every agent here too, besides `server/scope.rs`, which keeps `/api/connectors/` the owner's): the calls, the block state, Block and Unblock.
use super::{calls, change::change};
use crate::app::{bad, Shared, R};
use axum::extract::{Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::{from_fn, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/connectors/calls", get(listed))
        .route(
            "/api/connectors/blocks",
            get(|State(app): State<Shared>| async move { Json(calls::blocks(&app)) })
                .post(|State(app): State<Shared>, Json(b): Json<Value>| changed(app, b, true))
                .delete(|State(app): State<Shared>, Json(b): Json<Value>| changed(app, b, false)),
        )
        .route_layer(from_fn(owner_only))
}

async fn owner_only(req: Request, next: Next) -> Response {
    let who = req.extensions().get::<crate::server::Who>();
    if !matches!(who, Some(crate::server::Who::Owner)) {
        let why = "the connectors' calls and blocks are the owner's alone";
        return (StatusCode::FORBIDDEN, why).into_response();
    }
    next.run(req).await
}

async fn listed(State(app): State<Shared>, Query(q): Query<HashMap<String, String>>) -> R {
    let field = |k: &str| q.get(k).map(String::as_str).unwrap_or("").to_string();
    Ok(Json(calls::calls(&app, &field("bot"), &field("connector"))))
}

async fn changed(app: Shared, b: Value, on: bool) -> R {
    let field = |k: &str| b[k].as_str().unwrap_or("").trim().to_string();
    let (id, bot) = (field("connector"), field("bot"));
    let work = move || change(&app, &id, &bot, on).map(|said| (said, calls::blocks(&app)));
    let (said, blocks) = tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| bad("the change stopped".into()))?
        .map_err(bad)?;
    Ok(Json(json!({"said": said, "blocks": blocks})))
}
