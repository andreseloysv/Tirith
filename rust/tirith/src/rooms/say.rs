//! The owner writes in a room (page or CLI).
use super::{outbox::send, store::known, store::room, store::update};
use crate::app::{bad, Shared, R};
use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};
/// It counts as the owner for the brake, and is sealed for the paired phones and the accepted managers.
pub(super) async fn api_say(
    State(app): State<Shared>,
    Path(id): Path<String>,
    Json(b): Json<Value>,
) -> R {
    let text = b["text"].as_str().unwrap_or("").trim().to_string();
    (!text.is_empty())
        .then_some(())
        .ok_or_else(|| bad("empty message".into()))?;
    known(&id)?;
    let r = update(&app, &id, |r| (r.brake.streak, r.brake.braked) = (0, false))
        .and(room(&id))
        .ok_or_else(|| bad("no such room".into()))?;
    let readers: Vec<String> = r
        .devices
        .iter()
        .chain(r.managers.iter().filter(|m| m.accepted))
        .map(|p| p.public.clone())
        .filter(|p| !p.is_empty())
        .collect(); // the paired phones and the accepted managers
    send(&app, &r, &text, &readers).await.map_err(bad)?;
    Ok(Json(json!({"ok": true})))
}
