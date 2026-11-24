//! The guide on the page: each agent's state, its setup started with one tap and run out of sight (no shell, no window), a code pasted from the browser, the sign-in page opened in the owner's browser.
pub mod capture;
mod chain;
mod input;
mod job;
mod list;
mod seen;
mod steps;

use crate::app::{bad, Shared, R};
use crate::server::Who;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

/// Every agent this build knows how to set up, and how far each is.
async fn agents(State(app): State<Shared>) -> R {
    let rows = tokio::task::spawn_blocking(move || list::rows(&app)).await;
    Ok(Json(json!({ "agents": rows.unwrap_or_default() })))
}

fn owner(who: &Who) -> Result<(), (axum::http::StatusCode, String)> {
    (!who.is_agent())
        .then_some(())
        .ok_or_else(|| bad("connecting agents is the owner's, not an agent's".into()))
}

/// Starts its setup; it runs on while the page asks how it goes.
async fn start(State(app): State<Shared>, Path(alias): Path<String>, who: Who) -> R {
    owner(&who)?;
    super::catalog::known(&alias).ok_or_else(|| bad(format!("no agent '{alias}'")))?;
    job::start(&alias).map_err(bad)?;
    tokio::spawn(chain::run(app, alias));
    Ok(Json(json!({"started": true})))
}

/// The code the browser showed, for the sign-in that waits for it.
async fn code(Path(alias): Path<String>, who: Who, Json(body): Json<Value>) -> R {
    owner(&who)?;
    let text = body["code"].as_str().unwrap_or("").trim().to_string();
    (!text.is_empty())
        .then_some(())
        .ok_or_else(|| bad("no code".into()))?;
    input::give(&alias, &text).map_err(bad)?;
    Ok(Json(json!({"given": true})))
}

/// Opens the sign-in page its login printed, in this computer's browser.
async fn open(Path(alias): Path<String>, who: Who) -> R {
    owner(&who)?;
    let link = job::get(&alias).map(|j| j.link).unwrap_or_default();
    link.starts_with("https://")
        .then_some(())
        .ok_or_else(|| bad("no sign-in page now".into()))?;
    let opened = tokio::task::spawn_blocking(move || crate::os::open(link.as_ref())).await;
    Ok(Json(json!({"opened": opened.unwrap_or(false)})))
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/setup/agents", get(agents))
        .route("/api/setup/agents/{alias}", post(start))
        .route("/api/setup/agents/{alias}/code", post(code))
        .route("/api/setup/agents/{alias}/open", post(open))
}

#[cfg(test)]
#[path = "../../../checks/setup_web.rs"]
mod tests;
