//! `POST /api/safari`: the lane and role come from the caller's token; `app` names the owner's browser (Safari when it names none).
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
    let which = b["app"].as_str().unwrap_or("");
    if super::uia::browser(which).is_some() {
        // the owner's Chrome or Edge, on Windows: the same roles, the same commands
        super::script::chat_role(&app, &agent.lane, &agent.role).map_err(bad)?;
        let out = super::uia_run::run(&app.home, which, &b)
            .await
            .map_err(bad)?;
        return Ok(match b["cmd"].as_str() {
            Some("read") => super::run::cut_page(&out),
            _ => out,
        });
    }
    safari(
        &app,
        &agent.lane,
        &agent.role,
        b["cmd"].as_str().unwrap_or(""),
        b["url"].as_str().unwrap_or(""),
        b["target"].as_str().unwrap_or(""),
        b["text"].as_str().unwrap_or(""),
    )
    .await
    .map_err(bad)
}
