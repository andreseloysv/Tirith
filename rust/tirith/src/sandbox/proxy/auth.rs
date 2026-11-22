//! Who is calling the proxy.
use crate::{app::App, sandbox::token};
use base64::Engine;

/// The caller of a request: whose the token in `Proxy-Authorization: Basic <anything>:token` is. The user name is for people to read.
pub fn caller(app: &App, headers: &[&str]) -> Option<String> {
    let value = headers
        .iter()
        .find_map(|h| {
            h.split_once(':')
                .filter(|(k, _)| k.trim().eq_ignore_ascii_case("proxy-authorization"))
        })?
        .1
        .trim();
    let (_, data) = value
        .split_once(' ')
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("basic"))?;
    let raw = String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(data.trim())
            .ok()?,
    )
    .ok()?;
    let (user, pass) = raw.rsplit_once(':')?;
    token::who_of(app, pass).or_else(|| token::retired(app, user, pass))
}
