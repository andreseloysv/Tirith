//! Signing in from a browser: `/auth/login` sends it to the provider (Authorization Code with PKCE, a public client)
//! and `/auth/callback` trades the code for the same access token the app brings, then opens the same session.
//! A cookie ties the state to the browser that began, so another's callback is refused (login CSRF).
use super::flow::{flow_cookie, held, login_path};
use super::reply::{failed, found};
use super::session::start_session;
use super::{pending, people, redeem, Settings};
use crate::app::Shared;
use crate::oauth::pkce;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use reqwest::Url;
use std::collections::HashMap;

type Asked = Query<HashMap<String, String>>;

pub(super) async fn begin(State(app): State<Shared>, Query(q): Asked) -> Response {
    let next = pending::safe_next(q.get("next").map_or("/", String::as_str));
    let (Some(s), Some(site)) = (Settings::of(&app), crate::server::public_url(&app)) else {
        let why = "browser sign-in needs [auth.oidc] and [server] public_url in config.toml";
        return failed(StatusCode::NOT_FOUND, why, "/", &flow_cookie(&app, "", 0));
    };
    let (verifier, state) = (pkce::verifier(), pkce::state());
    let mut url = Url::parse(&s.authorize_url).unwrap_or_else(|_| Url::parse("http://x/").unwrap());
    url.query_pairs_mut()
        .append_pair("client_id", &s.client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &format!("{site}/auth/callback"))
        .append_pair("scope", &s.request_scope())
        .append_pair("state", &state)
        .append_pair("code_challenge", &pkce::challenge(&verifier))
        .append_pair("code_challenge_method", "S256")
        .append_pair("response_mode", "query");
    pending::add(&state, verifier, next);
    found(url.as_str(), &[flow_cookie(&app, &state, 600)])
}

pub(super) async fn back(State(app): State<Shared>, h: HeaderMap, Query(q): Asked) -> Response {
    let (end, ask) = (flow_cookie(&app, "", 0), |k: &str| {
        q.get(k).map_or("", String::as_str)
    });
    let refused = |code: StatusCode, why: &str, next: &str| {
        app.event(
            "",
            "",
            "auth",
            "refused",
            &format!("sign-in refused: {why}"),
            "",
        );
        failed(code, why, &login_path(next), &end)
    };
    let state = ask("state");
    let (Some(s), Some(site)) = (Settings::of(&app), crate::server::public_url(&app)) else {
        return refused(StatusCode::NOT_FOUND, "browser sign-in is not set up", "/");
    };
    let entry = Some(state).filter(|v| !v.is_empty() && held(&app, &h).as_deref() == Some(v));
    let Some(entry) = entry.and_then(pending::take) else {
        let why = "this sign-in did not begin in this browser, or it is too old or was used";
        return refused(StatusCode::FORBIDDEN, why, "/");
    };
    if !ask("error").is_empty() {
        let said = format!(
            "the provider said {}: {}",
            ask("error"),
            ask("error_description")
        );
        return refused(StatusCode::FORBIDDEN, &said, &entry.next);
    }
    let redirect = format!("{site}/auth/callback");
    let got = redeem::token(&s, ask("code"), &entry.verifier, &redirect).await;
    let opened = match got {
        Ok(token) => start_session(&app, &s, &token).await,
        Err(why) => Err(why),
    };
    match opened {
        Ok((_, session)) => found(&entry.next, &[people::cookie(&app, &session, s.hours), end]),
        Err(why) => refused(StatusCode::FORBIDDEN, &why, &entry.next),
    }
}
