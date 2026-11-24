//! A package's hash and signature: the hash is of everything but the signature, the signature is the creator's over the hash.
use crate::rooms::sealed::{key_id, sha_hex, verify, Me};
use serde_json::{json, Value};

/// The package without its signature, as bytes: serde_json keeps object keys in order, so the same package has the same bytes.
pub(super) fn hash(p: &Value) -> String {
    let mut body = p.clone();
    body.as_object_mut().map(|o| o.remove("sig"));
    sha_hex(body.to_string().as_bytes())
}

/// The package signed by `me`, its creator.
pub(super) fn signed(mut p: Value, me: &Me) -> Value {
    p["creator"] = json!({"key": me.public, "id": me.id});
    let h = hash(&p);
    p["sig"] = json!(me.sign(&h));
    p
}

/// The creator's key id when the signature holds over this very package, else None.
pub(super) fn creator(p: &Value) -> Option<String> {
    let key = p["creator"]["key"].as_str()?;
    let ok = verify(&hash(p), p["sig"].as_str()?, key);
    (ok && p["creator"]["id"] == key_id(key)).then(|| key_id(key))
}
