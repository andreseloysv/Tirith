//! Tirith's own page is never a rehearsal's step: there a click would be the owner's yes.

/// Whether `url` is this daemon's page, however its address is written (a name, 127.x, a number: a browser sends all of them as loopback).
pub(super) fn own_page(url: &str, port: u16) -> bool {
    let url = url.trim().to_lowercase().replace('\\', "/");
    let rest = url.split_once("://").map_or(url.as_str(), |(_, r)| r);
    let place = rest.trim_start_matches('/');
    let place = place.split(['/', '?', '#']).next().unwrap_or("");
    let place = place.rsplit_once('@').map_or(place, |(_, h)| h);
    let Some((host, said)) = place.rsplit_once(':') else {
        return false;
    };
    let host = host.trim_end_matches('.');
    let local = host == "localhost"
        || host.ends_with(".localhost")
        || host.starts_with('[')
        || host.starts_with("0x")
        || host.contains('%')
        || host.chars().all(|c| c.is_ascii_digit() || c == '.');
    local && said.parse() == Ok(port)
}
