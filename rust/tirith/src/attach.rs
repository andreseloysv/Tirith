//! Chat attachments: files and images the owner drops, pastes or picks in the chat, saved under
//! `$TIRITH_HOME/attachments/<id>/<name>` so the manager can open them by path.

use crate::app::Shared;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{body::Bytes, http::StatusCode, Json, Router};
use rand::Rng;
use serde_json::{json, Value};
use std::collections::HashMap;

const LIMIT: usize = 25 * 1024 * 1024;

/// The text appended to the owner's message so the manager can open the attached files (absolute paths).
pub fn describe(paths: &[String]) -> String {
    let list: String = paths.iter().map(|p| format!("\n- {p}")).collect();
    if paths.is_empty() {
        list
    } else {
        format!("\n\nAttached files (open them with your file-reading tool):{list}")
    }
}

/// A safe file name: the last path part, only letters, digits, `.`, `-`, `_`; no leading dots.
fn sanitize(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let s: String = base.chars().map(|c| if c.is_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' }).collect();
    let s: String = s.trim_start_matches('.').chars().take(120).collect();
    Some(s).filter(|s| !s.is_empty()).unwrap_or_else(|| "file".into())
}

fn content_type(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or("").to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "json" => "application/json",
        "txt" | "md" | "log" | "csv" | "rs" | "py" | "js" | "ts" | "toml" | "yaml" | "yml" | "sh" | "html" | "htm" => "text/plain; charset=utf-8", // html too: never rendered as a page on this origin
        _ => "application/octet-stream",
    }
}

async fn upload(State(app): State<Shared>, Query(q): Query<HashMap<String, String>>, body: Bytes) -> Result<Json<Value>, (StatusCode, String)> {
    if body.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "empty file".into()));
    }
    let name = sanitize(q.get("name").map(String::as_str).unwrap_or(""));
    let id = format!("{:016x}", rand::thread_rng().gen::<u64>());
    let dir = app.home.join("attachments").join(&id);
    let path = dir.join(&name);
    let err = |e: std::io::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    tokio::fs::create_dir_all(&dir).await.map_err(err)?;
    tokio::fs::write(&path, &body).await.map_err(err)?;
    Ok(Json(json!({"id": id, "name": name, "path": path.display().to_string(), "url": format!("/api/attachments/{id}/{name}")})))
}

async fn fetch(State(app): State<Shared>, Path((id, name)): Path<(String, String)>) -> Response {
    let valid = !id.is_empty() && id.chars().all(|c| c.is_ascii_hexdigit()) && sanitize(&name) == name;
    match if valid { tokio::fs::read(app.home.join("attachments").join(&id).join(&name)).await.ok() } else { None } {
        Some(bytes) => ([("content-type", content_type(&name)), ("x-content-type-options", "nosniff"), ("content-security-policy", "sandbox")], bytes).into_response(),
        None => (StatusCode::NOT_FOUND, "no such attachment").into_response(),
    }
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/attachments", post(upload).layer(DefaultBodyLimit::max(LIMIT))).route("/api/attachments/{id}/{name}", get(fetch))
}
