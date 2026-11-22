//! The short, secret-free summary of a call's arguments an approval card shows: each top-level key with a short value; a value under a key that names a secret, or holding one of the connector's secret values, is never shown.
use serde_json::Value;

const KEYS: usize = 8;
const VALUE: usize = 60;
const ALL: usize = 400;
const HIDDEN: &str = "(hidden)";
/// A key with one of these in its name holds something secret.
const SECRET_WORDS: [&str; 8] = [
    "token",
    "secret",
    "password",
    "passwd",
    "key",
    "auth",
    "cookie",
    "credential",
];

/// The first `n` characters of `s`, with `…` when it was longer.
fn cut(s: &str, n: usize) -> String {
    match s.char_indices().nth(n) {
        Some((at, _)) => format!("{}…", &s[..at]),
        None => s.to_string(),
    }
}

/// Whether `s` holds one of the secret values.
fn holds(s: &str, secrets: &[String]) -> bool {
    secrets
        .iter()
        .any(|x| x.len() >= 4 && s.contains(x.as_str()))
}

/// One value, short; `secrets` are the connector's secret values for this bot.
fn shown(key: &str, v: &Value, secrets: &[String]) -> String {
    let key = key.to_lowercase();
    match v {
        _ if SECRET_WORDS.iter().any(|w| key.contains(w)) => HIDDEN.into(),
        _ if holds(&v.to_string(), secrets) => HIDDEN.into(),
        Value::String(s) => format!("\"{}\"", cut(s, VALUE)),
        Value::Array(a) => format!("[{} items]", a.len()),
        Value::Object(m) => format!("{{{} keys}}", m.len()),
        other => cut(&other.to_string(), VALUE),
    }
}

/// The summary of `args`, with no value of `secrets` in it.
pub fn summary(args: &Value, secrets: &[String]) -> String {
    let said = match args {
        Value::Object(m) if m.is_empty() => "no arguments".to_string(),
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            let mut parts: Vec<String> = keys
                .iter()
                .take(KEYS)
                .map(|k| (k, shown(k, &m[*k], secrets)))
                .map(|(k, v)| match holds(k, secrets) {
                    true => format!("{HIDDEN}: {v}"),
                    false => format!("{}: {v}", cut(k, 30)),
                })
                .collect();
            let more = keys.len().saturating_sub(KEYS);
            let _ = (more > 0).then(|| parts.push(format!("and {more} more")));
            parts.join(", ")
        }
        other => shown("", other, secrets),
    };
    cut(&said, ALL)
}
