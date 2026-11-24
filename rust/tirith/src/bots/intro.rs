//! The welcome a new owner gets once the first agent connects (web/js/views/onboarding/): the first bot's opening
//! words written into its chat as its own, so the conversation goes on there with them as its context, and the
//! owner's first name as the computer knows it, for the greeting.
use crate::app::{bad, Shared, R};
use crate::server::Who;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

/// At most this many opening lines, each cut at this many characters.
const LINES: usize = 12;
const CHARS: usize = 4000;

async fn intro(
    State(app): State<Shared>,
    Path(id): Path<String>,
    who: Who,
    Json(body): Json<Value>,
) -> R {
    (!who.is_agent())
        .then_some(())
        .ok_or_else(|| bad("the welcome is the owner's".into()))?;
    let project = format!("bot-{id}");
    let known = app.cfg().factory.role::<String>(&project, "name").is_some();
    known
        .then_some(())
        .ok_or_else(|| bad(format!("no bot {id}")))?;
    let texts = body["texts"].as_array().cloned().unwrap_or_default();
    let lines: Vec<String> = texts
        .iter()
        .filter_map(|t| t.as_str())
        .take(LINES)
        .map(|t| t.chars().take(CHARS).collect())
        .collect();
    lines
        .iter()
        .for_each(|text| app.chat("manager", text, &project));
    Ok(Json(json!({ "written": lines.len() })))
}

/// `{name}`: the owner's first name, or null.
async fn me() -> R {
    let name = tokio::task::spawn_blocking(crate::os::full_name)
        .await
        .ok()
        .flatten();
    let first = name.and_then(|n| n.split_whitespace().next().map(String::from));
    Ok(Json(json!({ "name": first })))
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/bots/{id}/intro", post(intro))
        .route("/api/me", get(me))
}
