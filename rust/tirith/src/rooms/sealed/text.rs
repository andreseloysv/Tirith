//! Hex, key ids, names and canonical JSON, as am.mjs writes them.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
pub fn sha_hex(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}
pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
pub(super) fn unhex(s: &str) -> Option<Vec<u8>> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}
pub fn key_id(public: &str) -> String {
    unhex(public).map(|b| sha_hex(&b)).unwrap_or_default()
}
/// `name` as the platform and `am.mjs` clean it.
pub fn clean(s: &str) -> String {
    static OTHER: OnceLock<regex::Regex> = OnceLock::new();
    let out = OTHER
        .get_or_init(|| regex::Regex::new("[^a-z0-9._-]+").unwrap())
        .replace_all(&s.to_lowercase(), "-")
        .into_owned();
    out.trim_start_matches(|c: char| !c.is_ascii_alphanumeric())
        .trim_end_matches('-')
        .chars()
        .take(40)
        .collect()
}
/// JSON with sorted keys and no spaces, as `canonical()` in am.mjs.
pub fn canonical(v: &Value) -> String {
    match v {
        Value::Array(a) => format!(
            "[{}]",
            a.iter().map(canonical).collect::<Vec<_>>().join(",")
        ),
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            format!(
                "{{{}}}",
                keys.iter()
                    .map(|k| format!("{}:{}", Value::from(k.as_str()), canonical(&m[*k])))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        _ => v.to_string(),
    }
}
