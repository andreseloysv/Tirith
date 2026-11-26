//! `GET /api/costs?project=<id>&since=<date>&by=<item|lane|run|issue|model|day|project>&format=<json|csv>`.
use super::{export, group::BY};
use crate::app::Shared;
use axum::extract::{Query, State};
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use std::collections::HashMap;

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/costs", get(costs))
}

async fn costs(State(app): State<Shared>, Query(q): Query<HashMap<String, String>>) -> Response {
    let get = |k: &str| q.get(k).cloned().unwrap_or_default();
    let by = Some(get("by"))
        .filter(|b| BY.contains(&b.as_str()))
        .unwrap_or("item".into());
    let report = export::report(&app, &get("project"), &get("since"), &by);
    match get("format").as_str() {
        "csv" => (
            [(CONTENT_TYPE, "text/csv; charset=utf-8")],
            export::csv(&report),
        )
            .into_response(),
        _ => Json(report).into_response(),
    }
}
