//! The cookie that ties a browser sign-in to the browser that began it, and the address to begin (again).
use super::people;
use crate::app::App;
use axum::http::HeaderMap;
use reqwest::Url;

pub(super) fn flow_cookie(app: &App, value: &str, secs: u64) -> String {
    let secure = if crate::server::public_https(app) {
        "; Secure"
    } else {
        ""
    };
    let name = format!("{}-login", people::cookie_name(app));
    format!("{name}={value}; Path=/auth; Max-Age={secs}; HttpOnly; SameSite=Lax{secure}")
}

/// Where to begin (again), with the page to come back to.
pub(super) fn login_path(next: &str) -> String {
    let url = Url::parse_with_params("http://x/auth/login", [("next", next)]).unwrap();
    format!("{}?{}", url.path(), url.query().unwrap_or(""))
}

pub(super) fn held(app: &App, h: &HeaderMap) -> Option<String> {
    let name = format!("{}-login=", people::cookie_name(app));
    let all = h.get_all("cookie").iter().filter_map(|v| v.to_str().ok());
    let pair = all
        .flat_map(|v| v.split(';'))
        .find_map(|p| p.trim().strip_prefix(&name));
    pair.map(String::from)
}
