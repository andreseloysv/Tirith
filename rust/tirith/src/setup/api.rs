//! The daemon's side of the guide: who it is, and the connection check.
use super::prove::prove;
use crate::app::{bad, Shared, R};
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;

async fn who(State(app): State<Shared>) -> R {
    app.reload_config(false); // the guide may just have changed it
    let home = app.home.canonicalize().unwrap_or_else(|_| app.home.clone());
    Ok(Json(
        json!({"home": home, "pid": std::process::id(), "version": crate::bundle::VERSION}),
    ))
}

async fn check(
    State(app): State<Shared>,
    Path(alias): Path<String>,
    h: axum::http::HeaderMap,
) -> R {
    (!crate::server::agent(&h))
        .then_some(())
        .ok_or_else(|| bad("connecting agents is the owner's, not an agent's".into()))?;
    prove(&app, &alias).await.map(Json).map_err(bad)
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/setup", get(who))
        .route("/api/setup/check/{alias}", post(check))
}
