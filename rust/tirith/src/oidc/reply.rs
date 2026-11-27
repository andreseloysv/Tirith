//! What a browser is answered in the sign-in: a 302 that sets cookies, or a small page that says why it failed.
use axum::http::{header, StatusCode};
use axum::response::{AppendHeaders, Html, IntoResponse, Response};

pub(super) fn found(to: &str, cookies: &[String]) -> Response {
    let set = cookies.iter().map(|c| (header::SET_COOKIE, c.clone()));
    let (status, to) = (StatusCode::FOUND, [(header::LOCATION, to.to_string())]);
    (status, to, AppendHeaders(set)).into_response()
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The reason, and a way to try again; `cookie` ends the sign-in's own cookie.
pub(super) fn failed(status: StatusCode, why: &str, again: &str, cookie: &str) -> Response {
    let why: String = why.chars().take(300).collect();
    let page = format!(
        "<!doctype html><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>NeoBot</title>\
<body style=\"font:16px/1.5 system-ui;margin:20vh auto;max-width:28rem;padding:0 1.5rem;color-scheme:light dark\">\
<h1>NeoBot</h1><p>Sign-in did not work: {}</p><p><a href=\"{}\">Try again</a></p>",
        escape(&why),
        escape(again)
    );
    (
        status,
        [(header::SET_COOKIE, cookie.to_string())],
        Html(page),
    )
        .into_response()
}
