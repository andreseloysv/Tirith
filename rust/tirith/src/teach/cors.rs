//! Who may call the teach API: the page (same origin) and Tirith's extension with the bot's token. CORS answers only the extension's origin.
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
/// The origin of Tirith's extension: fixed by the `key` in tools/teach/manifest.json, so an unpacked install has this id.
pub const ORIGIN: &str = "chrome-extension://bogleilobfephnbnjibgoofoeipkhfef";
fn origin(h: &HeaderMap) -> Option<&str> {
    h.get("origin").and_then(|v| v.to_str().ok())
}
/// Whether the request came from the extension.
pub(super) fn from_extension(h: &HeaderMap) -> bool {
    origin(h) == Some(ORIGIN)
}
/// Whether `Authorization: Bearer <token>` carries the bot's token (compared in constant time).
pub(super) fn has_token(h: &HeaderMap, token: &str) -> bool {
    let sent = h
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    sent.len() == token.len()
        && !token.is_empty()
        && sent
            .bytes()
            .zip(token.bytes())
            .fold(0u8, |d, (a, b)| d | (a ^ b))
            == 0
}
/// Whether the call carries an agent's identity: a lane's or a role's (what `tirith` run from an agent's session sends). Only the owner, from the page, carries none.
pub fn agent_call(h: &HeaderMap) -> bool {
    ["x-tirith-lane", "x-tirith-role"].iter().any(|k| {
        h.get(*k)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| !v.is_empty() && !v.starts_with('$'))
    })
}
/// 401 for a call without the token.
pub(super) fn refused() -> Response {
    (StatusCode::UNAUTHORIZED, "no token, or not this bot's").into_response()
}
/// The answer with CORS headers for the extension's origin, and none for any other.
pub(super) fn allow(h: &HeaderMap, res: impl IntoResponse) -> Response {
    let mut res = res.into_response();
    if from_extension(h) {
        let out = res.headers_mut();
        out.insert(
            "access-control-allow-origin",
            HeaderValue::from_static(ORIGIN),
        );
        out.insert("vary", HeaderValue::from_static("Origin"));
    }
    res
}
/// The preflight: the extension may POST and GET with its token and JSON.
pub(super) async fn preflight(h: HeaderMap) -> Response {
    let mut res = allow(&h, StatusCode::NO_CONTENT);
    if from_extension(&h) {
        let out = res.headers_mut();
        let pairs = [
            ("access-control-allow-methods", "GET, POST, OPTIONS"),
            (
                "access-control-allow-headers",
                "authorization, content-type",
            ),
            ("access-control-max-age", "600"),
        ];
        for (k, v) in pairs {
            out.insert(k, HeaderValue::from_static(v));
        }
    }
    res
}
