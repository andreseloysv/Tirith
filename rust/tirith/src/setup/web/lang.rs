//! The owner's language (English, Spanish or German), as the page sets it: kept in kv, so every bot and role is
//! told to write to the owner in it (`manager/prompt.rs`).
use crate::app::{bad, Shared, R};
use crate::server::Who;
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

const KEY: &str = "owner_language";

/// The owner's language as the bots are told it; none for English, the factory's own.
pub fn told(app: &Shared) -> Option<&'static str> {
    match app.kv(KEY).as_deref() {
        Some("es") => Some("Spanish (español)"),
        Some("de") => Some("German (Deutsch)"),
        _ => None,
    }
}

pub(super) async fn set(State(app): State<Shared>, who: Who, Json(body): Json<Value>) -> R {
    super::owner(&who)?;
    let lang = body["lang"].as_str().unwrap_or("");
    matches!(lang, "en" | "es" | "de")
        .then_some(())
        .ok_or_else(|| bad("the languages are en, es and de".into()))?;
    app.set_kv(KEY, Some(lang).filter(|l| *l != "en"));
    Ok(Json(json!({ "lang": lang })))
}
