//! Who is calling the proxy.
use crate::sandbox::token::token;
use base64::Engine;

/// The caller of a request: the `who` of a valid `Proxy-Authorization: Basic who:token`.
pub fn caller(headers: &[&str]) -> Option<String> {
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
    let who = user
        .replace("%3A", ":")
        .replace("%3a", ":")
        .replace('~', ":");
    (pass == token(&who)).then_some(who)
}
