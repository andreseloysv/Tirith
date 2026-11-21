//! `POST /api/safari`: the lane and role come from the caller's headers.
use super::run::safari;
use crate::app::{bad, Shared};
use axum::{extract::State, Json};
use serde_json::Value;

pub async fn safari_route(
    State(app): State<Shared>,
    headers: axum::http::HeaderMap,
    Json(b): Json<Value>,
) -> Result<String, (axum::http::StatusCode, String)> {
    let h = |k: &str| {
        headers
            .get(k)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string()
    };
    safari(
        &app,
        &h("x-tirith-lane"),
        &h("x-tirith-role"),
        b["cmd"].as_str().unwrap_or(""),
        b["url"].as_str().unwrap_or(""),
    )
    .await
    .map_err(bad)
}
