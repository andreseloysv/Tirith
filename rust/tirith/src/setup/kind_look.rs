//! How a value looks: the URL of a public provider or of a private host, a key by its prefix or its shape.
use super::kind::Kind;

/// Hosts of public providers: no secret, whatever their variable is called.
const PUBLIC_HOSTS: [&str; 7] = [
    "api.anthropic.com",
    "api.openai.com",
    "api.githubcopilot.com",
    "api.mistral.ai",
    "openrouter.ai",
    "generativelanguage.googleapis.com",
    "github.com",
];
const PREFIXES: [&str; 8] = [
    "sk-",
    "ghp_",
    "gho_",
    "ghs_",
    "github_pat_",
    "xox",
    "AKIA",
    "AIza",
];

pub(super) fn url(value: &str) -> Kind {
    let rest = value.split_once("://").map_or("", |(_, r)| r);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains('@') || rest.contains('?') {
        return Kind::Secret; // user info or a query may carry a key
    }
    let host = authority
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if PUBLIC_HOSTS.contains(&host.as_str()) {
        Kind::Open
    } else {
        Kind::Host(host)
    }
}

pub(super) fn looks_like_key(value: &str) -> bool {
    if PREFIXES.iter().any(|p| value.starts_with(p)) && value.len() >= 16 {
        return true;
    }
    let plain = value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    let digit = value.bytes().any(|b| b.is_ascii_digit());
    let upper = value.bytes().any(|b| b.is_ascii_uppercase());
    let lower = value.bytes().any(|b| b.is_ascii_lowercase());
    plain && value.len() >= 32 && digit && upper && lower
}
