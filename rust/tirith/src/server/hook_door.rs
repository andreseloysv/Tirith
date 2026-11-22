//! The door of a Codex under its caller's limit (sandbox/nested/): the rules' hook and nothing else. That limit's profile refuses the daemon's own port in the kernel, so no request of the page or an agent pays for a look at who calls.
use super::{hook::rules_alone, origin};
use crate::app::Shared;
use axum::http::StatusCode;
use axum::{middleware, routing::post, Router};

const WHY: &str = "a command under Codex's sandbox limit has nothing of Tirith but the rules' hook: say in your answer what is needed";

pub(super) fn routes(app: Shared) -> Router {
    Router::new()
        .route("/hook/{event}", post(rules_alone))
        .fallback(|| async { (StatusCode::FORBIDDEN, WHY) })
        .layer(middleware::from_fn(origin::same_origin))
        .with_state(app)
}

/// Opens it on a loopback port of its own, for this daemon's lifetime: that port.
pub fn rules_door(app: &Shared) -> Result<u16, String> {
    let open = || -> std::io::Result<(tokio::net::TcpListener, u16)> {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        Ok((tokio::net::TcpListener::from_std(listener)?, port))
    };
    let (listener, port) = open().map_err(|e| format!("no door for the rules' hook: {e}"))?;
    let router = routes(app.clone());
    tokio::spawn(async move { axum::serve(listener, router).await });
    Ok(port)
}
