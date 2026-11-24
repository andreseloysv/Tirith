//! Who may act for a bot in the teach API: the page, or that bot by its own token.
use super::api::key;
use super::cors::from_extension;
use crate::app::Shared;
use crate::server::Who;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// The bot's conversation, for a call from the page or the bot's `neobot teach`, never from the extension nor another agent.
pub(super) fn own(
    app: &Shared,
    who: &Who,
    h: &HeaderMap,
    id: &str,
) -> Result<String, Box<Response>> {
    match key(app, h, id, false) {
        Ok(_) if from_extension(h) => Err(Box::new(StatusCode::FORBIDDEN.into_response())),
        Ok(k) if who.is_agent() && who.agent().is_none_or(|a| a.bot != k) => {
            let e = "a bot rehearses in its own conversation, never in another's name";
            let said = (StatusCode::FORBIDDEN, Json(json!({"error": e})));
            Err(Box::new(said.into_response()))
        }
        Ok(k) => Ok(k),
        Err(r) => Err(r),
    }
}
