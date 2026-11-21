//! A rehearsal's screenshots: kept as files in the bot's folder (`rehearsals/`), where the bot can look at them, and served to the page.
use super::api::key;
use super::cors::from_extension;
use crate::app::Shared;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use std::path::PathBuf;
fn dir(app: &Shared, key: &str) -> PathBuf {
    crate::bots::folder(app, key).join("rehearsals")
}
/// Keeps the screenshot the extension sent (a `data:image/jpeg` or `png` URL): its file name and full path; None when it is not one.
pub(super) fn save(app: &Shared, key: &str, data: &str) -> Option<(String, PathBuf)> {
    let (head, b64) = data.split_once(";base64,")?;
    let ext = match head {
        "data:image/jpeg" => "jpg",
        "data:image/png" => "png",
        _ => return None,
    };
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    let name = format!("{}.{ext}", chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f"));
    let path = dir(app, key).join(&name);
    std::fs::create_dir_all(dir(app, key)).ok()?;
    std::fs::write(&path, bytes).ok()?;
    Some((name, path))
}
/// A file name the daemon wrote: digits, dashes and one extension.
fn ours(name: &str) -> bool {
    let (stem, ext) = name.rsplit_once('.').unwrap_or(("", ""));
    matches!(ext, "jpg" | "png")
        && !stem.is_empty()
        && stem.bytes().all(|c| c.is_ascii_digit() || c == b'-')
}
/// `GET /api/teach/<id>/shot/<file>`: one screenshot, to the page only.
pub(super) async fn shot(
    State(app): State<Shared>,
    h: HeaderMap,
    Path((id, name)): Path<(String, String)>,
) -> Response {
    let key = match key(&app, &h, &id, false) {
        Ok(k) if !from_extension(&h) && ours(&name) => k,
        Ok(_) => return StatusCode::FORBIDDEN.into_response(),
        Err(r) => return *r,
    };
    let kind = if name.ends_with(".png") {
        "image/png"
    } else {
        "image/jpeg"
    };
    match std::fs::read(dir(&app, &key).join(&name)) {
        Ok(bytes) => ([(header::CONTENT_TYPE, kind)], bytes).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
