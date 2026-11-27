//! The routes of team sign-in: what the app needs to start (`GET /api/auth/oidc`), the token's trade for a session
//! (`POST /api/auth/oidc`), the one-use link that hands that session to the app's window, who am I and sign out.
use super::browser;
use super::login::{enter, login};
use super::{people, Settings};
use crate::app::Shared;
use crate::server::Who;
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/auth/oidc", get(config).post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(me))
        .route("/auth/session/{code}", get(enter))
        .route("/auth/login", get(browser::begin))
        .route("/auth/callback", get(browser::back))
}

/// The routes that stand before the gate: they prove who calls themselves.
pub fn own_route(method: &Method, path: &str) -> bool {
    (path == "/api/auth/oidc" && (method == Method::GET || method == Method::POST))
        || (method == Method::GET
            && (path.starts_with("/auth/session/")
                || path == "/auth/login"
                || path == "/auth/callback"))
}

/// What the app needs to sign a person in: the provider's endpoints and the scope to ask.
async fn config(State(app): State<Shared>) -> Response {
    let Some(s) = Settings::of(&app) else {
        return (
            StatusCode::NOT_FOUND,
            "team sign-in is not set up: [auth.oidc] in config.toml",
        )
            .into_response();
    };
    let urls = json!({"authorization_endpoint": s.authorize_url, "token_endpoint": s.token_url});
    let scope = s.request_scope();
    Json(json!({"issuer": s.issuer, "client_id": s.client_id, "scope": scope, "endpoints": urls}))
        .into_response()
}

async fn logout(State(app): State<Shared>, h: HeaderMap) -> Response {
    let name = people::cookie_name(&app);
    let own = h
        .get_all("cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'));
    own.filter_map(|p| p.trim().strip_prefix(&format!("{name}=")))
        .for_each(|t| people::end_session(&app, t));
    (
        [("set-cookie", people::cookie(&app, "", 0))],
        Json(json!({"ok": true})),
    )
        .into_response()
}

/// Who the gate took the caller for.
async fn me(who: Who) -> Json<Value> {
    Json(match who {
        Who::Person(p) => {
            json!({"role": p.level.name(), "name": p.name, "username": p.username, "oid": p.oid})
        }
        Who::Owner => json!({"role": "owner"}),
        _ => json!({"role": "none"}),
    })
}
