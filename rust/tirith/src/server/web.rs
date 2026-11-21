//! The page's files.
use crate::app::Shared;
use crate::attach;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// The page: the files of `web/` (`[app] web_dir` of config.toml, else this repository's), read from disk on each request so a change to the page shows on reload. Nothing outside that folder is ever served.
pub(super) async fn web_file(State(app): State<Shared>, uri: axum::http::Uri) -> Response {
    let rel = Some(uri.path().trim_start_matches('/'))
        .filter(|r| !r.is_empty())
        .unwrap_or("index.html");
    let Some(()) = (!rel
        .split('/')
        .any(|p| p.is_empty() || p == ".." || p.starts_with('.')))
    .then_some(()) else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    let dir = crate::config::get::<String>(&app.cfg().raw, &["app", "web_dir"])
        .map(|d| crate::config::expand(&d))
        .unwrap_or_else(|| {
            std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../web"))
        });
    let Ok(body) = tokio::fs::read(dir.join(rel)).await else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    let kind = attach::content_kind(rel, true);
    (
        [("content-type", kind), ("cache-control", "no-cache")],
        body,
    )
        .into_response()
}
