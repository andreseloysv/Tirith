//! The owner's 👍/👎 on a turn: `POST /api/turns/<id>/reaction` with `{"value": 1, -1 or 0}`, or a message in a bot's conversation that is only 👍 or 👎 (or +1 / -1).
use super::signals::last;
use crate::app::Shared;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use rusqlite::params;
use serde_json::{json, Value};

/// The reaction a message is when it is only 👍 or 👎 (any skin tone), or +1 / -1.
pub fn reaction_of(text: &str) -> Option<i64> {
    let bare: String = text
        .trim()
        .chars()
        .filter(|c| !matches!(c, '\u{FE0F}' | '\u{1F3FB}'..='\u{1F3FF}'))
        .collect();
    match bare.as_str() {
        "👍" | "+1" => Some(1),
        "👎" | "-1" => Some(-1),
        _ => None,
    }
}

/// Keeps `value` (1, -1, or 0 to take it back: none) as the reaction of turn `id`.
pub fn react(app: &Shared, id: i64, value: i64) -> Result<(), String> {
    let Some(()) = matches!(value, -1..=1).then_some(()) else {
        return Err("a reaction is 1, -1 or 0".into());
    };
    let known = app.query("SELECT id FROM turns WHERE id = ?1", [id], |r| {
        r.get::<_, i64>(0)
    });
    let Some(()) = (!known.is_empty()).then_some(()) else {
        return Err(format!("no turn {id}"));
    };
    let value = Some(value).filter(|v| *v != 0);
    app.insert(
        "UPDATE turns SET reaction = ?1 WHERE id = ?2",
        params![value, id],
    );
    app.emit("turn", json!({"id": id, "reaction": value}));
    Ok(())
}

/// The owner's message `text` in conversation `conv` is a reaction to its bot's last turn: kept on that turn, and true, so the caller starts no turn and judges nothing (the chat row stays the caller's).
pub fn reacted(app: &Shared, conv: &str, text: &str) -> bool {
    let Some(value) = reaction_of(text).filter(|_| crate::bots::bot(app, conv).is_some()) else {
        return false;
    };
    last(app, conv).is_some_and(|id| react(app, id, value).is_ok())
}

async fn reaction(
    State(app): State<Shared>,
    Path(id): Path<i64>,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let value = body["value"].as_i64().unwrap_or(2);
    match react(&app, id, value) {
        Ok(()) => (StatusCode::OK, Json(json!({"id": id, "reaction": value}))),
        Err(e) if e.starts_with("no turn") => (StatusCode::NOT_FOUND, Json(json!({"error": e}))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e}))),
    }
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/turns/{id}/reaction", post(reaction))
}
