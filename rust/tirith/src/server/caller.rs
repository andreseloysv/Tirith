//! Who calls: the owner (the page, their phone's line, a terminal of theirs) or an agent. An agent names itself (`tirith`, its hooks), and a caller inside Tirith's sandbox is one whatever it sends: the kernel says so (`sandbox/caller.rs`) and this marks its request. The owner's yes never comes from an agent.
use super::door::Door;
use axum::extract::{connect_info::Connected, ConnectInfo, Request};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::serve::IncomingStream;
use std::net::SocketAddr;

/// The daemon's own mark on a request from inside the sandbox: a caller's own is dropped.
pub(crate) const CONFINED: &str = "x-tirith-confined";
/// The chat's other speakers: an agent that calls itself one of them is just an agent.
const SPEAKERS: [&str; 6] = ["owner", "room", "tirith", "card", "tool", "routine"];
const OWNERS: &str =
    "that is the owner's, from the page or their own terminal, never an agent's: say in your chat what you need and end your turn";

/// Both ends of a caller's connection.
#[derive(Clone, Copy)]
pub struct Caller(SocketAddr, SocketAddr);

impl Connected<IncomingStream<'_, Door>> for Caller {
    fn connect_info(stream: IncomingStream<'_, Door>) -> Self {
        let from = *stream.remote_addr();
        Caller(from, stream.io().local_addr().unwrap_or(from))
    }
}

/// The agent that calls (its lane or role, `agent` for a confined caller that names none); empty for the owner.
pub fn agent_name(h: &HeaderMap) -> &str {
    let named = ["x-tirith-lane", "x-tirith-role"].iter().find_map(|k| {
        h.get(*k)
            .and_then(|v| v.to_str().ok())
            .filter(|v| !v.is_empty() && !v.starts_with('$'))
    });
    match named {
        Some(name) if !SPEAKERS.contains(&name) => name,
        None if !h.contains_key(CONFINED) => "",
        _ => "agent",
    }
}

/// Whether an agent calls.
pub fn agent(h: &HeaderMap) -> bool {
    !agent_name(h).is_empty()
}

/// Where the answer depends on who calls (asking the kernel costs some milliseconds, so only there).
pub(super) fn asked(method: &Method, path: &str) -> bool {
    if unblocks(method, path) {
        return true;
    }
    let reads = ["/api/teach/", "/api/rooms"];
    let changes = ["/api/owner/", "/api/setup/check/"];
    reads.iter().any(|p| path.starts_with(p))
        || method != Method::GET
            && (changes.iter().any(|p| path.starts_with(p))
                || path == "/api/chat"
                || path.ends_with("/library")
                || path.ends_with("/reaction"))
}

/// Lifting one of the owner's network blocks.
fn unblocks(method: &Method, path: &str) -> bool {
    method == Method::DELETE && path == "/api/net/blocks"
}

/// What no agent does at all: pairing a phone, which is then the owner, the owner's 👍 or 👎, and lifting their blocks.
pub(super) fn owners_alone(method: &Method, path: &str) -> bool {
    let pairs =
        path.starts_with("/api/rooms/") && (path.ends_with("/qr") || path.ends_with("/qr.svg"));
    pairs
        || unblocks(method, path)
        || (path.starts_with("/api/turns/") && path.ends_with("/reaction"))
}

/// Marks a confined caller, and refuses an agent what is the owner's alone.
pub(super) async fn mark(mut req: Request, next: Next) -> Response {
    req.headers_mut().remove(CONFINED);
    let path = req.uri().path().to_string();
    if !asked(req.method(), &path) {
        return next.run(req).await;
    }
    let ends = req.extensions().get::<ConnectInfo<Caller>>().map(|c| c.0);
    if let Some(Caller(from, to)) = ends {
        let look = tokio::task::spawn_blocking(move || crate::sandbox::confined(from, to));
        if look.await.unwrap_or(true) {
            req.headers_mut()
                .insert(CONFINED, HeaderValue::from_static("1"));
        }
    }
    if owners_alone(req.method(), &path) && agent(req.headers()) {
        return (StatusCode::FORBIDDEN, OWNERS).into_response();
    }
    next.run(req).await
}
