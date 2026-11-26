//! The page behind a TLS reverse proxy on the same machine: `[server] public_url = "https://neobot.team.example"`
//! lets that host (and that origin) in beside the loopback ones. X-Forwarded-* is read only when a public URL is set
//! and the connection is from a loopback address, which every connection to this daemon is, the proxy's included.
use crate::app::App;
use axum::http::HeaderMap;

/// `[server] public_url`, without a trailing slash.
pub fn url(app: &App) -> Option<String> {
    let url: String = crate::config::get(&app.cfg().raw, &["server", "public_url"])?;
    Some(url.trim_end_matches('/').to_string())
        .filter(|u| u.starts_with("http://") || u.starts_with("https://"))
}

pub fn public_https(app: &App) -> bool {
    url(app).is_some_and(|u| u.starts_with("https://"))
}

/// Host and origin of the public URL.
fn parts(url: &str) -> (&str, &str) {
    (url.split_once("://").map_or(url, |(_, h)| h), url)
}

/// The host the browser asked for: X-Forwarded-Host when a public URL is set and the proxy is on this machine, else Host.
pub(super) fn asked_host<'a>(app: &App, h: &'a HeaderMap, from_loopback: bool) -> Option<&'a str> {
    let named = |n: &str| h.get(n).map(|v| v.to_str().unwrap_or("\u{0}"));
    let forwarded = url(app)
        .filter(|_| from_loopback)
        .and_then(|_| named("x-forwarded-host"));
    forwarded
        .map(|f| f.split(',').next().unwrap_or("").trim())
        .or(named("host"))
}

/// Whether `host` and `origin` are the public URL's.
pub(super) fn is_public(app: &App, host: Option<&str>, origin: Option<&str>) -> bool {
    let Some(url) = url(app) else { return false };
    let (own_host, own_origin) = parts(&url);
    host.is_none_or(|h| h.eq_ignore_ascii_case(own_host)) && origin.is_none_or(|o| o == own_origin)
}
