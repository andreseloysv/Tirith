//! A bot made from a template: its card, its routine, its skill and its first words.
use super::make::Parts;
use super::template::{templates, Template};
use crate::app::{bad, Shared, R};
use crate::server::s;
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

/// A new bot from the template `id`.
pub(super) fn create(app: &Shared, id: &str) -> Result<Value, String> {
    let t = templates(app)?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or(format!("no bot template '{id}'"))?;
    super::make::make(
        app,
        Parts {
            name: &t.name,
            shape: &t.shape,
            color: &t.color,
            routines: vec![t.routine.clone()],
            skills: vec![(t.id.clone(), t.skill.clone())],
            start: &t.start,
            origin: "template",
        },
    )
}

async fn list(State(app): State<Shared>) -> R {
    let all = templates(&app).map_err(bad)?;
    Ok(Json(json!(all
        .iter()
        .map(Template::shown)
        .collect::<Vec<_>>())))
}

async fn make(State(app): State<Shared>, Json(b): Json<Value>) -> R {
    create(&app, &s(&b, "template")).map(Json).map_err(bad)
}

pub(super) fn routes() -> Router<Shared> {
    Router::new().route("/api/bots/templates", get(list)).route(
        "/api/bots/from-template",
        post(make).route_layer(axum::middleware::from_fn(super::owner::owner_only)),
    )
}
