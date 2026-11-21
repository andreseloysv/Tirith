//! The attachments' HTTP API.
use super::{fetch::fetch, upload::upload};
use crate::app::Shared;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;

const LIMIT: usize = 25 * 1024 * 1024;

pub fn routes() -> Router<Shared> {
    Router::new()
        .route(
            "/api/attachments",
            post(upload).layer(DefaultBodyLimit::max(LIMIT)),
        )
        .route("/api/attachments/{id}/{name}", get(fetch))
}
