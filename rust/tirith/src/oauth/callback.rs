//! The daemon's 127.0.0.1 callback, the one place a code comes in: the state must be a sign-in under way (taken once), the code is exchanged with its verifier, the tokens kept in that bot's store. The browser is shown only whether it worked.
use super::{keep::keep, pending, token};
use crate::app::Shared;
use axum::extract::{Query, State};
use axum::http::{Method, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use std::collections::HashMap;

pub(super) const PATH: &str = "/oauth/callback";

/// Whether the request is the callback, whose handler checks a state of its own instead of a token.
pub fn own_route(method: &Method, path: &str) -> bool {
    method == Method::GET && path == PATH
}

fn page(code: StatusCode, said: &str) -> Response {
    let html = format!("<!doctype html><meta charset=\"utf-8\"><title>Tirith</title><body style=\"font:16px/1.5 system-ui;margin:20vh auto;max-width:28rem;padding:0 1.5rem;color-scheme:light dark\"><h1>Tirith</h1><p>{said}</p>");
    (code, Html(html)).into_response()
}

pub(super) async fn callback(
    State(app): State<Shared>,
    who: crate::server::Who,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let state = q.get("state").map(String::as_str).unwrap_or_default();
    if matches!(who, crate::server::Who::Agent(_)) {
        return page(
            StatusCode::FORBIDDEN,
            "A sign-in comes from the owner's browser alone.",
        );
    }
    let Some(p) = pending::take(state).filter(|_| !state.is_empty()) else {
        return page(
            StatusCode::BAD_REQUEST,
            "This sign-in is unknown or over: run tirith connector login again.",
        );
    };
    let done = match (q.get("code"), q.get("error")) {
        (Some(code), None) => {
            let tokens = token::exchange(&p.grant, code, &p.verifier, &p.redirect).await;
            match tokens {
                Ok(t) => keep(&app, (&p.bot, &p.id), t, p.grant).await.map(drop),
                Err(why) => Err(why),
            }
        }
        (_, error) => Err(format!(
            "the server said {}",
            error.map_or("no code", |e| e)
        )),
    };
    let said = match &done {
        Ok(()) => format!(
            "{} is signed in for bot {}. You can close this tab.",
            p.id, p.bot
        ),
        Err(_) => "The sign-in failed: the terminal that started it says why.".into(),
    };
    let ok = done.is_ok();
    let what = format!("connector {} signed in for bot {}", p.id, p.bot);
    let _ = ok.then(|| app.event("", "", "info", "gateway", &what, ""));
    pending::end(p.ticket, done);
    page(
        if ok {
            StatusCode::OK
        } else {
            StatusCode::BAD_GATEWAY
        },
        &said,
    )
}
