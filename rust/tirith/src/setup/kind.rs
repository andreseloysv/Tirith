//! What one auth variable is: a secret, a private host, or nothing to hide (a model id, a flag, a public provider's URL).
use super::kind_look as look;

pub(super) enum Kind {
    Secret,
    Host(String),
    Open,
}

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

pub(super) fn of(var: &str, value: &str) -> Kind {
    let name = var.to_ascii_uppercase();
    if name.ends_with("_HOST") {
        return Kind::Host(value.to_string());
    }
    let looks = look::looks_like_key(value);
    let named = name.split('_').any(|w| SECRET_NAMES.contains(&w));
    if looks || (named && value.len() >= 12) {
        return Kind::Secret;
    }
    if value.starts_with("http://") || value.starts_with("https://") {
        return look::url(value);
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
