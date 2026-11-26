//! A team member at the gate: the session cookie `oidc/` gave them, held to what their role allows (`policy.rs`).
//! A refusal is a 403 that says what is missing, and an event.
use super::who::Who;
use super::{page, policy, refused};
use crate::app::Shared;
use crate::oidc::{self, Person};
use axum::http::{HeaderMap, Method};
use axum::response::Response;

/// The person whose live session this request carries (and, for the API, not another site's request).
pub(super) fn of(app: &Shared, path: &str, h: &HeaderMap) -> Option<Person> {
    let name = oidc::cookie_name(app);
    let cookies = h
        .get_all("cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'));
    let token = cookies
        .filter_map(|p| p.trim().strip_prefix(&format!("{name}=")))
        .next()?;
    page::fetch_ok(path, h).then_some(())?;
    oidc::by_session(app, token)
}

/// None when `who` may make this call; else the refusal, already an event. Every change a person makes is one too.
pub(super) fn held(app: &Shared, who: &Who, method: &Method, path: &str) -> Option<Response> {
    let Who::Person(p) = who else { return None };
    let need = policy::need(app, method, path);
    let by = format!("{} ({})", p.name, p.level.name());
    if !need.allows(p.level) {
        let said = format!("{by} may not {method} {path}: it needs {}", need.name());
        app.event("", "", "auth", "refused", &said, "");
        return Some(refused::role(p.level.name(), need.name()));
    }
    if method != Method::GET {
        app.event(
            "",
            "",
            "auth",
            "action",
            &format!("{by}: {method} {path}"),
            "",
        );
    }
    None
}
