//! The gate in front of every route: who calls comes from a token and from nothing else, and whoever has none gets 403. Two kinds of route stand apart: the agents' hooks, where no token leaves only the refusals, and those with a token of their own (the teach extension's, the page's one-use link, a connector sign-in's state on its callback).
use super::who::{sent, Who};
use super::{caller, identify, page, person, refused, scope};
use crate::app::Shared;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, Method};
use axum::middleware::Next;
use axum::response::Response;

pub(super) async fn admit(State(app): State<Shared>, mut req: Request, next: Next) -> Response {
    if req.uri().path().starts_with("/door/") {
        return next.run(req).await; // the model door asks for its own token (modeldoor/route.rs)
    }
    let (method, path) = (req.method().clone(), req.uri().path().to_string());
    let (from, headers) = (caller::of(&req), req.headers().clone());
    let who = if is_hook(&method, &path) {
        identify::agent(&app, &headers).map_or(Who::Nobody, Who::Agent)
    } else if sent(&headers).is_some() {
        match by_token(&app, &headers, &from).await {
            Who::Nobody => return refused::nobody(&path),
            who => who,
        }
    } else if carries_its_own(&app, &method, &path, &headers) {
        if !caller::outside(&from).await {
            return refused::nobody(&path);
        }
        Who::Extension
    } else if page::opened(&app, &path, &headers) && caller::outside(&from).await {
        Who::Owner
    } else if let Some(p) = person::of(&app, &path, &headers) {
        Who::Person(p)
    } else if let Some(go) = crate::oidc::sign_in_redirect(&app, &method, req.uri(), &headers) {
        return go;
    } else {
        return refused::nobody(&path);
    };
    if let Some(refusal) = person::held(&app, &who, &method, &path) {
        return refusal;
    }
    if let Who::Agent(agent) = &who {
        if let Some(why) = scope::refused(agent, &method, &path) {
            return refused::not_theirs(why);
        }
    }
    req.extensions_mut().insert(who);
    next.run(req).await
}

/// The owner by `owner.token`, never from inside the sandbox; else the agent the token is.
async fn by_token(app: &Shared, headers: &HeaderMap, from: &Option<caller::Caller>) -> Who {
    if page::same(sent(headers).unwrap_or(""), &app.keys.owner) {
        return if caller::outside(from).await {
            Who::Owner
        } else {
            Who::Nobody
        };
    }
    identify::agent(app, headers).map_or(Who::Nobody, Who::Agent)
}

/// `POST /hook/<event>`, exactly: anything longer is another route.
fn is_hook(method: &Method, path: &str) -> bool {
    method == Method::POST
        && path
            .strip_prefix("/hook/")
            .is_some_and(|event| !event.is_empty() && !event.contains('/'))
}

/// The routes whose handler checks a token of its own.
fn carries_its_own(app: &Shared, method: &Method, path: &str, headers: &HeaderMap) -> bool {
    page::link(method, path)
        || crate::teach::own_token(app, method, path, headers)
        || crate::oauth::own_route(method, path)
        || crate::oidc::own_route(method, path)
}
