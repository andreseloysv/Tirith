//! The teach routes that carry a token of their own: the bot's, which makes its holder the extension.
use super::cors::{from_extension, has_token};
use crate::app::Shared;
use axum::http::{HeaderMap, Method};

/// True for a request the handler's own token check answers (`api.rs`): the extension's routes always, reading, starting and stopping with the bot's token or from the extension's origin, and that origin's preflights (they carry nothing).
pub fn own_token(app: &Shared, method: &Method, path: &str, h: &HeaderMap) -> bool {
    let Some(rest) = path.strip_prefix("/api/teach/") else {
        return false;
    };
    if method == Method::OPTIONS {
        return from_extension(h);
    }
    let (id, route) = rest.split_once('/').unwrap_or((rest, ""));
    let (get, post) = (method == Method::GET, method == Method::POST);
    match route {
        "events" | "done" if post => true,
        "next" if get => true,
        "" if get => extension(app, id, h),
        "start" | "stop" if post => extension(app, id, h),
        _ => false,
    }
}

/// A browser may send no origin with a GET: the token then says it is the extension.
fn extension(app: &Shared, id: &str, h: &HeaderMap) -> bool {
    from_extension(h)
        || h.contains_key("authorization")
            && crate::bots::key_for(app, id)
                .is_some_and(|key| has_token(h, &super::state::token(app, &key)))
}
