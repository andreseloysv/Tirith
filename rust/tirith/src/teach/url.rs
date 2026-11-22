//! A recorded address with its secrets masked: the password of `user:password@`, and the value of a secret query or fragment parameter (code, token, key, password, sig, session and the like); ordinary ones stay.
use super::mask::MASK;
use super::names::words;
/// Words that make a parameter secret.
const NAMES: [&str; 25] = [
    "code",
    "token",
    "tokens",
    "key",
    "password",
    "passwd",
    "pwd",
    "pass",
    "passcode",
    "secret",
    "sig",
    "signature",
    "session",
    "sid",
    "auth",
    "otp",
    "jwt",
    "bearer",
    "credential",
    "credentials",
    "apikey",
    "ticket",
    "assertion",
    "pin",
    "cvv",
];
/// Longer names that hold one (`accesstoken`, `sessionid`).
const INSIDE: [&str; 6] = [
    "token",
    "password",
    "secret",
    "apikey",
    "session",
    "signature",
];
fn secret(name: &str) -> bool {
    words(name)
        .iter()
        .any(|w| NAMES.contains(&w.as_str()) || INSIDE.iter().any(|i| w.contains(i)))
}
/// `a=1&token=x` with the value of each secret parameter masked (an empty value stays).
fn params(q: &str) -> String {
    let each = q.split('&').map(|p| match p.split_once('=') {
        Some((n, v)) if !v.is_empty() && secret(n) => format!("{n}={MASK}"),
        _ => p.to_string(),
    });
    each.collect::<Vec<_>>().join("&")
}
/// The fragment: parameters after a `?` (`#/login?token=x`), or the whole of it when it is `a=b&c=d`.
fn fragment(f: &str) -> String {
    match f.split_once('?') {
        Some((route, q)) => format!("{route}?{}", params(q)),
        None if f.contains('=') => params(f),
        None => f.to_string(),
    }
}
/// `scheme://user:password@host/path` without the password.
fn userinfo(base: &str) -> String {
    let (scheme, rest) = base.split_once("://").unwrap_or(("", base));
    let (auth, path) = rest.find('/').map_or((rest, ""), |i| rest.split_at(i));
    match auth.rsplit_once('@').map(|(c, h)| (c.split_once(':'), h)) {
        Some((Some((user, _)), host)) if !scheme.is_empty() => {
            format!("{scheme}://{user}:{MASK}@{host}{path}")
        }
        _ => base.to_string(),
    }
}
/// The address with its secrets masked.
pub(super) fn mask_url(url: &str) -> String {
    let (rest, frag) = url
        .split_once('#')
        .map_or((url, None), |(a, b)| (a, Some(b)));
    let (base, query) = rest
        .split_once('?')
        .map_or((rest, None), |(a, b)| (a, Some(b)));
    let mut out = userinfo(base);
    let _ = query.map(|q| out.push_str(&format!("?{}", params(q))));
    let _ = frag.map(|f| out.push_str(&format!("#{}", fragment(f))));
    out
}
