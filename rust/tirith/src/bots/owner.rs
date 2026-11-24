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
