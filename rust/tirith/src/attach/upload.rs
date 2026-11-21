//! Saving an uploaded attachment in its own folder.
use super::name::sanitize;
use crate::app::Shared;
use axum::extract::{Query, State};
use axum::{body::Bytes, http::StatusCode, Json};
use rand::Rng;
use serde_json::{json, Value};
use std::collections::HashMap;

pub(super) async fn upload(
    State(app): State<Shared>,
    Query(q): Query<HashMap<String, String>>,
    body: Bytes,
) -> Result<Json<Value>, (StatusCode, String)> {
    (!body.is_empty())
        .then_some(())
        .ok_or((StatusCode::BAD_REQUEST, "empty file".to_string()))?;
    let name = sanitize(q.get("name").map(String::as_str).unwrap_or(""));
    let id = format!("{:016x}", rand::thread_rng().gen::<u64>());
    let dir = app.home.join("attachments").join(&id);
    let path = dir.join(&name);
    let err = |e: std::io::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    tokio::fs::create_dir_all(&dir).await.map_err(err)?;
    tokio::fs::write(&path, &body).await.map_err(err)?;
    Ok(Json(
        json!({"id": id, "name": name, "path": path.display().to_string(), "url": format!("/api/attachments/{id}/{name}")}),
    ))
}
