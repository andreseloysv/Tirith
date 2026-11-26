//! What one auth variable is: a secret, a private host, or nothing to hide (a model id, a flag, a public provider's URL).

pub(super) enum Kind {
    Secret,
    Host(String),
    Open,
}

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
const OPEN_NAMES: [&str; 6] = [
    "_MODEL",
    "_TYPE",
    "_OFFLINE",
    "_EFFORT",
    "_PROVIDER",
    "_WIRE_API",
];
const SECRET_NAMES: [&str; 11] = [
    "KEY",
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "PASSWD",
    "CREDENTIAL",
    "AUTH",
    "COOKIE",
    "BEARER",
    "SESSION",
    "PAT",
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

pub(super) fn of(var: &str, value: &str) -> Kind {
    let name = var.to_ascii_uppercase();
    if name.ends_with("_HOST") {
        return Kind::Host(value.to_string());
    }
    let looks = looks_like_key(value);
    let named = name.split('_').any(|w| SECRET_NAMES.contains(&w));
    if looks || (named && value.len() >= 12) {
        return Kind::Secret;
    }
    if value.starts_with("http://") || value.starts_with("https://") {
        return url(value);
    }
    let open = OPEN_NAMES.iter().any(|n| name.ends_with(n)) || is_flag(value);
    if open || named || value.len() < 12 {
        return Kind::Open;
    }
    Kind::Secret // an unknown name keeps the conservative default
}

fn is_flag(value: &str) -> bool {
    let v = value.to_ascii_lowercase();
    ["true", "false", "0", "1", "yes", "no", "on", "off"].contains(&v.as_str())
}

fn url(value: &str) -> Kind {
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

fn looks_like_key(value: &str) -> bool {
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
