//! `POST /api/safari`: the lane and role come from the caller's token.
use super::run::safari;
use crate::app::{bad, Shared};
use axum::{extract::State, Json};
use serde_json::Value;

pub async fn safari_route(
    State(app): State<Shared>,
    who: crate::server::Who,
    Json(b): Json<Value>,
) -> Result<String, (axum::http::StatusCode, String)> {
    let agent = who.agent().cloned().unwrap_or_default();
    safari(
        &app,
        &agent.lane,
        &agent.role,
        b["cmd"].as_str().unwrap_or(""),
        b["url"].as_str().unwrap_or(""),
    )
    .await
    .map_err(bad)
}
