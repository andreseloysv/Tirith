//! The sandbox's HTTP routes: its status, the block list, the destinations seen and Safari.
use super::{
    blocks::{block, list},
    profile::{enabled, proxy_port},
    proxy,
    safari::safari_route,
};
use crate::app::{bad, Shared, R};
use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};

pub fn routes() -> Router<Shared> {
    Router::new()
        .route(
            "/api/net/status",
            get(|State(app): State<Shared>| async move { Json(status(&app)) }),
        )
        .route(
            "/api/net/blocks",
            get(|| async { Json(json!(list())) })
                .post(|State(app): State<Shared>, Json(b): Json<Value>| change(app, b, true))
                .delete(|State(app): State<Shared>, Json(b): Json<Value>| change(app, b, false)),
        )
        .route(
            "/api/net/destinations",
            get(|| async { Json(proxy::destinations()) }),
        )
        .route("/api/safari", axum::routing::post(safari_route))
}

fn status(app: &Shared) -> Value {
    json!({"sandbox": enabled(app).is_ok(), "why": enabled(app).err(), "proxy_port": proxy_port(app),
    "proxy": proxy::state(), "blocks": list()})
}

async fn change(app: Shared, b: Value, on: bool) -> R {
    let rule = block(&app, b["rule"].as_str().unwrap_or(""), on).map_err(bad)?;
    Ok(Json(json!({"rule": rule, "blocks": list()})))
}
