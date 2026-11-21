//! The shape of a block rule: a domain, an address or a CIDR range.
use std::net::IpAddr;

/// `https://api.example.com:443/x` -> `api.example.com`; `*.x.com` -> `x.com`; addresses and ranges stay.
pub fn clean(rule: &str) -> String {
    let t = rule.trim().to_lowercase();
    let t = t.split_once("://").map_or(t.as_str(), |(_, rest)| rest);
    let t = if cidr(t).is_none() {
        t.split(['/', '?', '#']).next().unwrap_or("")
    } else {
        t
    };
    let t = if t.matches(':').count() == 1 {
        t.split(':').next().unwrap_or("")
    } else {
        t
    };
    t.trim_start_matches("*.")
        .trim_matches(['.', '[', ']'])
        .to_string()
}

/// An address or a CIDR range: the network address and its prefix length.
pub fn cidr(text: &str) -> Option<(IpAddr, u32)> {
    let (a, b) = text.split_once('/').unwrap_or((text, ""));
    let ip: IpAddr = a.trim_matches(['[', ']']).parse().ok()?;
    let max = if ip.is_ipv4() { 32 } else { 128 };
    let bits = if b.is_empty() {
        max
    } else {
        b.parse().ok().filter(|n| *n <= max)?
    };
    Some((ip, bits))
}

pub fn in_range(ip: IpAddr, (net, bits): (IpAddr, u32)) -> bool {
    let mask = |w: u32| {
        if bits == 0 {
            0
        } else {
            u128::MAX << (w - bits)
        }
    };
    match (ip, net) {
        (IpAddr::V4(a), IpAddr::V4(n)) => {
            (u32::from(a) as u128 ^ u32::from(n) as u128) & mask(32) & 0xffff_ffff == 0
        }
        (IpAddr::V6(a), IpAddr::V6(n)) => (u128::from(a) ^ u128::from(n)) & mask(128) == 0,
        (IpAddr::V6(a), IpAddr::V4(_)) => a
            .to_ipv4_mapped()
            .is_some_and(|v4| in_range(IpAddr::V4(v4), (net, bits))),
        _ => false,
    }
}
