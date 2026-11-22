//! Canonical JSON (keys sorted at every depth, no spaces) and its sha256: what a record keeps in place of the content.
use serde_json::Value;

/// `v` written one way only, whatever order its keys came in.
pub fn canonical(v: &Value) -> String {
    match v {
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            let pairs: Vec<String> = keys
                .into_iter()
                .map(|k| format!("{}:{}", Value::String(k.clone()), canonical(&m[k])))
                .collect();
            format!("{{{}}}", pairs.join(","))
        }
        Value::Array(a) => {
            let items: Vec<String> = a.iter().map(canonical).collect();
            format!("[{}]", items.join(","))
        }
        other => other.to_string(),
    }
}

/// The sha256 (hex) of `v`'s canonical form.
pub fn sha(v: &Value) -> String {
    crate::rooms::sealed::sha_hex(canonical(v).as_bytes())
}
