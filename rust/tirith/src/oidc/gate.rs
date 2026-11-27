//! The gate's answer to a browser that opens a page without a session, on a server with a public address and a
//! provider: sign in there and come back. Everything else without a caller stays a 403.
use super::{flow::login_path, reply::found, Settings};
use crate::app::App;
use axum::http::{HeaderMap, Method, Uri};
use axum::response::Response;

pub fn sign_in_redirect(app: &App, method: &Method, uri: &Uri, h: &HeaderMap) -> Option<Response> {
    let path = uri.path();
    let api = ["/api/", "/hook/", "/door/", "/auth/"];
    let html = h
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|a| a.contains("text/html"));
    let wanted = method == Method::GET && html && !api.iter().any(|p| path.starts_with(p));
    let ready = Settings::of(app).is_some() && crate::server::public_url(app).is_some();
    let next = uri.path_and_query().map_or(path, |p| p.as_str());
    (wanted && ready).then(|| found(&login_path(next), &[]))
}
