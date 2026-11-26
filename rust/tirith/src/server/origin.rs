//! The same-origin guard in front of every route.
use crate::app::Shared;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::sync::OnceLock;

static PORT: OnceLock<u16> = OnceLock::new();

pub(super) fn set_port(port: u16) {
    let _ = PORT.set(port);
}

/// A browser page from another origin may not call the API (no CSRF against 127.0.0.1). It also checks Host, so a DNS-rebinding page (Host: evil.example) cannot read the API either. NeoBot's own extension may call the teach API only, where its token is checked. Behind a reverse proxy (`[server] public_url`) the public host and origin are welcome too.
pub(super) async fn same_origin(State(app): State<Shared>, req: Request, next: Next) -> Response {
    let ok = allowed(&app, &req);
    if ok {
        next.run(req).await
    } else {
        (StatusCode::FORBIDDEN, "foreign origin").into_response()
    }
}

fn allowed(app: &Shared, req: &Request) -> bool {
    let h = req.headers();
    let extension = req.uri().path().starts_with("/api/teach/")
        && h.get("origin").is_some_and(|o| o == crate::teach::ORIGIN);
    let text = |n: &str| h.get(n).map(|v| v.to_str().unwrap_or("\u{0}"));
    let local = super::caller::of(req).is_none_or(|c| c.loopback());
    let host = super::public::asked_host(app, h, local);
    let port = PORT.get().copied().unwrap_or(0);
    origin_host_ok(text("host"), text("origin"), port)
        || local && super::public::is_public(app, host, text("origin"))
        || extension && host_ok(h.get("host"), port)
}

fn host_ok(host: Option<&axum::http::HeaderValue>, port: u16) -> bool {
    origin_host_ok(host.map(|v| v.to_str().unwrap_or("\u{0}")), None, port)
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
