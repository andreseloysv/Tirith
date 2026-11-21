//! Chat attachments: files and images the owner drops, pastes or picks in the chat, saved under `$TIRITH_HOME/attachments/<id>/<name>` so the manager can open them by path.

use crate::app::Shared;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{body::Bytes, http::StatusCode, Json, Router};
use rand::Rng;
use serde_json::{json, Value};
use std::collections::HashMap;

const LIMIT: usize = 25 * 1024 * 1024;
/// Content kinds by file extension, `ext:kind|…` (the ones only a page file gets are named in `content_kind`).
const KINDS: &str = "png:image/png|jpg:image/jpeg|jpeg:image/jpeg|gif:image/gif|webp:image/webp|svg:image/svg+xml|pdf:application/pdf|json:application/json|html:text/html; charset=utf-8|css:text/css; charset=utf-8|js:application/javascript; charset=utf-8|mjs:application/javascript; charset=utf-8|woff2:font/woff2";

/// The Content-Type of a file named `name`: as a page file (`page`), or as an attachment, where text-shaped files (html too) are plain text, never rendered as a page on this origin, and page-only kinds (css, mjs, woff2) do not apply.
pub fn content_kind(name: &str, page: bool) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    let plain = "txt md log csv rs py js ts toml yaml yml sh html htm".split(' ').any(|e| e == ext);
    match (page, plain) {
        (false, true) => "text/plain; charset=utf-8",
        _ => KINDS.split('|').filter_map(|kv| kv.split_once(':')).find(|(e, _)| *e == ext && (page || !matches!(*e, "html" | "css" | "mjs" | "woff2"))).map(|(_, k)| k).unwrap_or("application/octet-stream"),
    }
}

/// The text appended to the owner's message so the manager can open the attached files (absolute paths).
pub fn describe(paths: &[String]) -> String {
    paths.is_empty().then(String::new).unwrap_or_else(|| format!("\n\nAttached files (open them with your file-reading tool):{}", paths.iter().map(|p| format!("\n- {p}")).collect::<String>()))
}

/// A safe file name: the last path part, only letters, digits, `.`, `-`, `_`; no leading dots.
fn sanitize(name: &str) -> String {
    let s: String = name.rsplit(['/', '\\']).next().unwrap_or("").chars().map(|c| if c.is_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' }).collect::<String>().trim_start_matches('.').chars().take(120).collect();
    Some(s).filter(|s| !s.is_empty()).unwrap_or_else(|| "file".into())
}

async fn fetch(State(app): State<Shared>, Path((id, name)): Path<(String, String)>) -> Response {
    let kind = content_kind(&name, false);
    let bytes = if !id.is_empty() && id.chars().all(|c| c.is_ascii_hexdigit()) && sanitize(&name) == name { tokio::fs::read(app.home.join("attachments").join(&id).join(&name)).await.ok() } else { None };
    bytes.map_or_else(|| (StatusCode::NOT_FOUND, "no such attachment").into_response(), |bytes| ([("content-type", kind), ("x-content-type-options", "nosniff"), ("content-security-policy", "sandbox")], bytes).into_response())
}

async fn upload(State(app): State<Shared>, Query(q): Query<HashMap<String, String>>, body: Bytes) -> Result<Json<Value>, (StatusCode, String)> {
    (!body.is_empty()).then_some(()).ok_or((StatusCode::BAD_REQUEST, "empty file".to_string()))?;
    let name = sanitize(q.get("name").map(String::as_str).unwrap_or(""));
    let id = format!("{:016x}", rand::thread_rng().gen::<u64>());
    let dir = app.home.join("attachments").join(&id);
    let path = dir.join(&name);
    let err = |e: std::io::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    tokio::fs::create_dir_all(&dir).await.map_err(err)?;
    tokio::fs::write(&path, &body).await.map_err(err)?;
    Ok(Json(json!({"id": id, "name": name, "path": path.display().to_string(), "url": format!("/api/attachments/{id}/{name}")})))
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/attachments", post(upload).layer(DefaultBodyLimit::max(LIMIT))).route("/api/attachments/{id}/{name}", get(fetch))
}
