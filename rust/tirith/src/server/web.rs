//! The page's files.
use crate::app::Shared;
use crate::{attach, bundle};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Where the page's tab «Enseñar» downloads the extension.
const EXTENSION: &str = "teach-extension.zip";

/// One file of the page: from its folder when there is one, else the binary's own copy.
async fn read(app: &Shared, rel: &str) -> Option<Vec<u8>> {
    let set = crate::config::get::<String>(&app.cfg().raw, &["app", "web_dir"]);
    let dir = set
        .map(|d| crate::config::expand(&d))
        .filter(|d| d.is_dir())
        .or_else(|| bundle::checkout("web"));
    match dir {
        Some(dir) => tokio::fs::read(dir.join(rel)).await.ok(),
        None => bundle::file(&format!("web/{rel}")).map(<[u8]>::to_vec),
    }
}

/// The page: the files of `web/` (`[app] web_dir` of config.toml, else the checkout a development build came from), read from disk on each request so a change to the page shows on reload; with neither folder, the files the binary carries. Nothing outside that folder is ever served.
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
    if rel == EXTENSION {
        let body = bundle::zip("tirith-teach", &bundle::below("tools/teach"));
        return ([("content-type", "application/zip")], body).into_response();
    }
    let Some(body) = read(&app, rel).await else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    let kind = attach::content_kind(rel, true);
    (
        [("content-type", kind), ("cache-control", "no-cache")],
        body,
    )
        .into_response()
}
