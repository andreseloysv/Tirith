//! The gateway's routes, for the bot run whose token opened it alone: open, the tools of now, one call, close. No answer carries a secret, a connector's command or its address.
use super::session::{close, find, keep};
use super::{call::call, list::tools, open};
use crate::app::{Shared, R};
use crate::server::Who;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/mcp/open", post(opened))
        .route("/api/mcp/{session}", axum::routing::delete(closed))
        .route("/api/mcp/{session}/tools", get(listed))
        .route("/api/mcp/{session}/call", post(called))
}

/// Whom the request's token is, exactly (`<role>@<conversation>`).
fn raw(app: &Shared, headers: &HeaderMap) -> Option<String> {
    let token = headers.get("x-tirith-token")?.to_str().ok()?;
    crate::sandbox::who_of(app, token)
}

fn gone() -> (StatusCode, String) {
    (
        StatusCode::NOT_FOUND,
        "no such gateway of this run: start neobot mcp again".into(),
    )
}

async fn opened(
    State(app): State<Shared>,
    who: Who,
    headers: HeaderMap,
    Json(b): Json<Value>,
) -> R {
    let bot = b["bot"].as_str().unwrap_or_default();
    let (key, raw) = open::proven(&app, &who, raw(&app, &headers), bot)?;
    let pid = b["pid"].as_u64().unwrap_or(0) as u32;
    let (s, failed) = open::open(&app, (bot, &key, &raw), pid).await;
    let listed = tools(&app, &s);
    let failed: Vec<&str> = failed.iter().filter_map(|f| f.split(": ").next()).collect(); // why is the owner's event
    let session = keep(s);
    Ok(Json(
        json!({"session": session, "tools": listed, "failed": failed}),
    ))
}

async fn listed(State(app): State<Shared>, headers: HeaderMap, Path(id): Path<String>) -> R {
    let s = find(&id, &raw(&app, &headers).unwrap_or_default()).ok_or_else(gone)?;
    Ok(Json(json!({"tools": tools(&app, &s)})))
}

async fn called(
    State(app): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(b): Json<Value>,
) -> R {
    let s = find(&id, &raw(&app, &headers).unwrap_or_default()).ok_or_else(gone)?;
    let args = b
        .get("arguments")
        .filter(|a| !a.is_null())
        .cloned()
        .unwrap_or(json!({}));
    Ok(Json(
        call(&app, &s, b["name"].as_str().unwrap_or(""), &args).await,
    ))
}

async fn closed(State(app): State<Shared>, headers: HeaderMap, Path(id): Path<String>) -> R {
    let ok = close(&id, &raw(&app, &headers).unwrap_or_default());
    ok.then(|| Json(json!({"ok": true}))).ok_or_else(gone)
}
