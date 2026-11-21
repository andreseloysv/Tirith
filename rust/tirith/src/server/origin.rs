//! The same-origin guard in front of every route.
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::sync::OnceLock;

static PORT: OnceLock<u16> = OnceLock::new();

pub(super) fn set_port(port: u16) {
    let _ = PORT.set(port);
}

/// A browser page from another origin may not call the API (no CSRF against 127.0.0.1). It also checks Host, so a DNS-rebinding page (Host: evil.example) cannot read the API either.
pub(super) async fn same_origin(req: Request, next: Next) -> Response {
    let ok = origin_host_ok(
        req.headers()
            .get("host")
            .map(|v| v.to_str().unwrap_or("\u{0}")),
        req.headers()
            .get("origin")
            .map(|v| v.to_str().unwrap_or("\u{0}")),
        PORT.get().copied().unwrap_or(0),
    );
    if ok {
        next.run(req).await
    } else {
        (StatusCode::FORBIDDEN, "foreign origin").into_response()
    }
}

pub(super) fn origin_host_ok(host: Option<&str>, origin: Option<&str>, port: u16) -> bool {
    let host_ok = host.is_none_or(|h| {
        ["127.0.0.1", "localhost"]
            .iter()
            .any(|n| h.eq_ignore_ascii_case(n) || h.eq_ignore_ascii_case(&format!("{n}:{port}")))
    });
    let origin_ok = origin.is_none_or(|o| {
        o == format!("http://127.0.0.1:{port}") || o == format!("http://localhost:{port}")
    });
    host_ok && origin_ok
}
