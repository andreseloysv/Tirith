//! Whether a destination is on a list: a host (exactly), `*.domain` (any subdomain of it), an address or a range.
use crate::sandbox::rules::{cidr, clean, in_range};
use std::net::IpAddr;

/// The destination as lists name it: lower case, no dots or brackets around.
pub(super) fn plain(host: &str) -> String {
    host.trim().trim_matches(['.', '[', ']']).to_lowercase()
}

/// `host` is one of `rules`' (`ask` is a word of the list, no destination).
pub(super) fn listed(rules: &[String], host: &str) -> bool {
    let ip: Option<IpAddr> = host.parse().ok();
    rules.iter().filter(|r| *r != "ask").any(|rule| {
        let rule = rule.trim().to_lowercase();
        match (ip, cidr(&rule)) {
            (Some(ip), Some(range)) => in_range(ip, range),
            (None, Some(_)) | (Some(_), None) => false,
            (None, None) => match rule.strip_prefix("*.") {
                Some(domain) => host.ends_with(&format!(".{}", clean(domain))),
                None => host == clean(&rule),
            },
        }
    })
}

/// The machine itself: no list holds a caller back from its own loopback.
pub(super) fn loopback(host: &str) -> bool {
    host == "localhost"
        || host
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.to_canonical().is_loopback())
}
