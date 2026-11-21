//! Serving a saved attachment, never as a page on this origin.
use super::{kind::content_kind, name::sanitize};
use crate::app::Shared;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub(super) async fn fetch(
    State(app): State<Shared>,
    Path((id, name)): Path<(String, String)>,
) -> Response {
    let kind = content_kind(&name, false);
    let bytes =
        if !id.is_empty() && id.chars().all(|c| c.is_ascii_hexdigit()) && sanitize(&name) == name {
            tokio::fs::read(app.home.join("attachments").join(&id).join(&name))
                .await
                .ok()
        } else {
            None
        };
    bytes.map_or_else(
        || (StatusCode::NOT_FOUND, "no such attachment").into_response(),
        |bytes| {
            (
                [
                    ("content-type", kind),
                    ("x-content-type-options", "nosniff"),
                    ("content-security-policy", "sandbox"),
                ],
                bytes,
            )
                .into_response()
        },
    )
}
