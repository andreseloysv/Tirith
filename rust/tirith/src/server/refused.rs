//! What a caller hears that the gate does not let in.
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// How the answer to a caller without a token begins; the guide knows a daemon by it.
pub const NO_CALLER: &str = "no valid caller token";

/// 403 for a caller nobody knows. A browser is shown how the owner's page opens.
pub(super) fn nobody(path: &str) -> Response {
    if path.starts_with("/api/") || path.starts_with("/hook/") {
        let said = format!("{NO_CALLER}: the owner's terminal has <home>/owner.token, an agent gets its own from NeoBot as it starts");
        return (StatusCode::FORBIDDEN, said).into_response();
    }
    let page = "<!doctype html><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>NeoBot</title>\
<body style=\"font:16px/1.5 system-ui;margin:20vh auto;max-width:28rem;padding:0 1.5rem;color-scheme:light dark\">\
<h1>NeoBot</h1><p>This browser is not signed in as the owner. Open the NeoBot app, or run:</p><p><code>neobot open</code></p>";
    (StatusCode::FORBIDDEN, Html(page)).into_response()
}

/// 403 for an agent on what is not its own.
pub(super) fn not_theirs(why: &'static str) -> Response {
    (StatusCode::FORBIDDEN, why).into_response()
}

/// 403 for a person whose role does not reach what they asked.
pub(super) fn role(has: &str, needs: &str) -> Response {
    let said = format!("forbidden: your role ({has}) may not do this, it needs {needs}");
    (StatusCode::FORBIDDEN, said).into_response()
}
