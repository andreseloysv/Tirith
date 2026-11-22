//! A new line in a conversation, from the owner's page or from an agent.
use super::s;
use crate::app::{bad, Shared, R};
use crate::{attach, manager};
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

pub(super) async fn chat(State(app): State<Shared>, who: super::Who, Json(b): Json<Value>) -> R {
    let text = s(&b, "text")
        + &attach::describe(
            &b["attachments"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|f| f.as_str().map(String::from))
                .collect::<Vec<_>>(),
        );
    (!text.trim().is_empty())
        .then_some(())
        .ok_or_else(|| bad("empty message".into()))?;
    manager::say(&app, &s(&b, "project"), &text, who.name())
        .map(|_| Json(json!({"ok": true})))
        .map_err(bad)
}
