//! `/door/<alias>/<path>`: who calls comes from the token the provider's SDK sends as its key; then the path, the budget and the forwarding.
use super::{forward, on, paths, record, spec};
use crate::app::Shared;
use axum::extract::{DefaultBodyLimit, Path, RawQuery, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{body::Bytes, routing::any, Json, Router};
use serde_json::json;

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/door/{alias}/{*rest}", any(call))
        .layer(DefaultBodyLimit::max(64 << 20))
}

pub(super) fn refuse(status: StatusCode, message: &str) -> Response {
    let body = json!({"type": "error", "error": {"type": "door", "message": message}});
    (status, Json(body)).into_response()
}

/// The token the SDK sent as its key: `x-api-key`, `api-key` or `Authorization: Bearer`.
fn sent(h: &HeaderMap) -> Option<&str> {
    let get = |n: &str| h.get(n).and_then(|v| v.to_str().ok());
    let bearer = get("authorization").map(|a| a.strip_prefix("Bearer ").unwrap_or(a));
    get("x-api-key").or(get("api-key")).or(bearer)
}

async fn call(
    State(app): State<Shared>,
    Path((alias, rest)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    body: Bytes,
) -> Response {
    let who = sent(&headers)
        .and_then(|t| crate::sandbox::who_of(&app, t))
        .and_then(|w| w.strip_prefix(&format!("door:{alias}:")).map(String::from));
    let Some(who) = who.filter(|_| on::enabled(&app)) else {
        let why = "this is NeoBot's model door: it takes the token its agent was given as the key";
        return refuse(StatusCode::UNAUTHORIZED, why);
    };
    let vars = crate::setup::auth_env(&app.home, &alias);
    let path = format!("/{rest}");
    let Some(p) = spec::find(&vars)
        .into_iter()
        .find(|p| path.starts_with(&format!("{}/", p.path)))
    else {
        return refuse(
            StatusCode::NOT_FOUND,
            &format!("no provider key is kept for {alias}"),
        );
    };
    let after = &path[p.path.len()..];
    if !paths::known(method.as_str(), after) {
        return refuse(
            StatusCode::FORBIDDEN,
            "the model door forwards a provider's model paths only",
        );
    }
    let role = crate::sandbox::role_of(&app, &who);
    if let Some(why) = record::over_budget(&app, &role) {
        return refuse(StatusCode::TOO_MANY_REQUESTS, &why);
    }
    forward::send(
        &app,
        forward::Call {
            alias,
            who,
            role,
            provider: p,
            path,
            query,
        },
        method,
        headers,
        body,
    )
    .await
}
