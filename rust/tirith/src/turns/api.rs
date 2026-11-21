//! `GET /api/turns?conv=<key>&limit=<n>`: a conversation's turns, the last first.
use crate::app::Shared;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use rusqlite::types::ValueRef;
use serde_json::{json, Value};
use std::collections::HashMap;

const COLS: [&str; 25] = [
    "id",
    "conv",
    "role",
    "trigger",
    "routine",
    "started_at",
    "ended_at",
    "ms",
    "status",
    "agent",
    "model",
    "effort",
    "tokens_in",
    "tokens_out",
    "tokens_cache_read",
    "tokens_cache_write",
    "cost_usd",
    "steps",
    "skills",
    "tools",
    "versions",
    "answer",
    "event",
    "factory",
    "role_hash",
];

/// One cell as JSON: a number, a text (the JSON columns parsed), or null.
fn cell(col: &str, v: ValueRef) -> Value {
    match v {
        ValueRef::Integer(n) => json!(n),
        ValueRef::Real(f) => json!(f),
        ValueRef::Text(t) => {
            let t = String::from_utf8_lossy(t);
            let parsed = matches!(col, "skills" | "tools" | "versions")
                .then(|| serde_json::from_str(&t).ok())
                .flatten();
            parsed.unwrap_or_else(|| json!(t))
        }
        _ => Value::Null,
    }
}

/// The last `limit` turns of conversation `conv` (of every conversation when empty), the last first.
pub fn turns(app: &Shared, conv: &str, limit: u32) -> Vec<Value> {
    let cols = COLS.map(|c| format!("\"{c}\"")).join(", ");
    let sql = format!(
        "SELECT {cols} FROM turns WHERE ?1 = '' OR conv = ?1 ORDER BY id DESC LIMIT {limit}"
    );
    app.query(&sql, [conv], |r| {
        let row = COLS
            .iter()
            .enumerate()
            .map(|(i, c)| r.get_ref(i).map(|v| (c.to_string(), cell(c, v))));
        row.collect::<rusqlite::Result<serde_json::Map<String, Value>>>()
            .map(Value::Object)
    })
}

pub fn routes() -> Router<Shared> {
    Router::new().route(
        "/api/turns",
        get(
            |State(app): State<Shared>, Query(q): Query<HashMap<String, String>>| async move {
                let conv = q.get("conv").cloned().unwrap_or_default();
                let limit = q.get("limit").and_then(|n| n.parse().ok()).unwrap_or(50);
                Json(json!(turns(&app, &conv, u32::min(limit, 1000))))
            },
        ),
    )
}
