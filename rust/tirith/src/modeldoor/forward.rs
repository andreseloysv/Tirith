//! Passing one call to the provider with the real key; its answer comes back through `back.rs`.
use super::build::{key_header, model, passes, CLIENT};
use super::record::{keep, Call as Row};
use super::{route::refuse, spec::Provider, tally::Tally};
use crate::app::Shared;
use axum::body::Bytes;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::Response;
use std::time::Instant;

/// What a call is: whose it is, which provider it goes to and where.
pub struct Call {
    pub alias: String,
    pub who: String,
    pub role: String,
    pub provider: Provider,
    pub path: String,
    pub query: Option<String>,
}

pub(super) async fn send(
    app: &Shared,
    c: Call,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let url = format!(
        "{}{}{}",
        c.provider.origin,
        c.path,
        c.query
            .as_deref()
            .map(|q| format!("?{q}"))
            .unwrap_or_default()
    );
    let mut up = CLIENT.request(method, &url).body(body.clone());
    for (name, value) in headers.iter().filter(|(n, _)| passes(n)) {
        up = up.header(name, value);
    }
    let (name, value) = key_header(&c.provider);
    let started = Instant::now();
    let row = Row {
        alias: c.alias.clone(),
        who: c.who.clone(),
        role: c.role.clone(),
        model: model(&c.path, &body),
        status: 502,
        ms: 0,
    };
    let Ok(answer) = up
        .header(name, value)
        .send()
        .await
        .map_err(|e| eprintln!("neobot: model door to {}: {e}", c.provider.origin))
    else {
        keep(app, &row, &Tally::default());
        return refuse(
            StatusCode::BAD_GATEWAY,
            "the model door could not reach the provider",
        );
    };
    super::back::back(app, answer, row, started)
}
