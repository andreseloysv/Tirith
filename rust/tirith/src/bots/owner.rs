//! What only the owner reaches: making a bot from outside (a template, a package) is refused to any agent's token.
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

pub(super) async fn owner_only(req: Request, next: Next) -> Response {
    let who = req.extensions().get::<crate::server::Who>();
    if !matches!(who, Some(crate::server::Who::Owner)) {
        return (
            StatusCode::FORBIDDEN,
            "making a bot from outside is the owner's alone",
        )
            .into_response();
    }
    next.run(req).await
}

/// A bot's connector grants are the owner's, and an admin's: they choose which connectors a bot may use, never a connector's sign-in.
pub(super) async fn admin_too(req: Request, next: Next) -> Response {
    let who = req.extensions().get::<crate::server::Who>();
    if !who.is_some_and(crate::server::Who::is_admin) {
        return (
            StatusCode::FORBIDDEN,
            "a bot's connectors are the owner's and an admin's",
        )
            .into_response();
    }
    next.run(req).await
}
