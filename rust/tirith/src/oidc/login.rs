//! The token's trade for a session, and the one-use link that hands the session to a window that cannot read the answer.
use super::{people, verify, Settings};
use crate::app::{now_iso, Shared};
use crate::rooms::sealed::{hex, sha_hex};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static LINKS: Mutex<Option<HashMap<String, (String, Instant)>>> = Mutex::new(None);

fn refuse(app: &Shared, code: StatusCode, why: &str) -> Response {
    app.event(
        "",
        "",
        "auth",
        "refused",
        &format!("sign-in refused: {why}"),
        "",
    );
    (code, Json(json!({ "error": why }))).into_response()
}

pub(super) async fn login(
    State(app): State<Shared>,
    h: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    let Some(s) = Settings::of(&app) else {
        return (
            StatusCode::NOT_FOUND,
            "team sign-in is not set up: [auth.oidc] in config.toml",
        )
            .into_response();
    };
    let bearer = h
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let sent = body.as_ref().and_then(|b| b["access_token"].as_str());
    let Some(token) = bearer.or(sent) else {
        return refuse(&app, StatusCode::BAD_REQUEST, "no access token");
    };
    let claims = match verify::check(&s, token).await {
        Ok(c) => c,
        Err(why) => return refuse(&app, StatusCode::FORBIDDEN, &why),
    };
    people::upsert(&app, &claims);
    let session = people::open_session(&app, &claims.oid, s.hours);
    let code = hex(&rand::random::<[u8; 16]>());
    let mut links = LINKS.lock().unwrap();
    let map = links.get_or_insert_with(HashMap::new);
    map.retain(|_, (_, at)| at.elapsed() < Duration::from_secs(60));
    map.insert(sha_hex(code.as_bytes()), (session.clone(), Instant::now()));
    drop(links);
    let said = format!(
        "{} ({}) signed in at {}",
        claims.name,
        claims.level.name(),
        now_iso()
    );
    app.event("", "", "auth", "login", &said, "");
    let body = json!({"name": claims.name, "username": claims.username, "role": claims.level.name(), "link": format!("/auth/session/{code}")});
    (
        [("set-cookie", people::cookie(&app, &session, s.hours))],
        Json(body),
    )
        .into_response()
}

/// The link hands the session to a browser or window that cannot read the response: its code works once.
pub(super) async fn enter(State(app): State<Shared>, Path(code): Path<String>) -> Response {
    let found = LINKS
        .lock()
        .unwrap()
        .as_mut()
        .and_then(|m| m.remove(&sha_hex(code.as_bytes())));
    let Some((session, _)) = found.filter(|(_, at)| at.elapsed() < Duration::from_secs(60)) else {
        return (
            StatusCode::FORBIDDEN,
            "this link was used or is too old: sign in again",
        )
            .into_response();
    };
    let hours = Settings::of(&app).map_or(12, |s| s.hours);
    let on = "<!doctype html><title>NeoBot</title><script>location.replace('/')</script>";
    (
        [("set-cookie", people::cookie(&app, &session, hours))],
        Html(on),
    )
        .into_response()
}
