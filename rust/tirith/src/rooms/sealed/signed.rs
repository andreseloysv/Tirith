//! Signatures and event ids: an event is genuine when its id, its author's signature and the platform's stamp all hold.
use super::text::{canonical, sha_hex, unhex};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde_json::{json, Value};
pub fn verify(message: &str, sig: &str, public: &str) -> bool {
    let (Ok(sig), Ok(public)) = (
        <[u8; 64]>::try_from(unhex(sig).unwrap_or_default()),
        <[u8; 32]>::try_from(unhex(public).unwrap_or_default()),
    ) else {
        return false;
    };
    VerifyingKey::from_bytes(&public).is_ok_and(|k| {
        k.verify(message.as_bytes(), &Signature::from_bytes(&sig))
            .is_ok()
    })
}
pub(super) fn curve_public(public: &str) -> Option<crypto_box::PublicKey> {
    let b: [u8; 32] = unhex(public)?.try_into().ok()?;
    Some(crypto_box::PublicKey::from(
        VerifyingKey::from_bytes(&b)
            .ok()?
            .to_montgomery()
            .to_bytes(),
    ))
}
pub fn event_id(e: &Value) -> String {
    let fields = json!({"author": e["author"], "body": e["body"], "kind": e["kind"], "refs": e["refs"], "tag": e["tag"], "ts": e["ts"]});
    sha_hex(canonical(&fields).as_bytes())
}
pub(super) fn inner_message(author: &str, tag: &str, text: &str) -> String {
    sha_hex(canonical(&json!({"author": author, "tag": tag, "text": text})).as_bytes())
}
/// The event matches its id, its author's signature and the platform's time stamp.
pub fn genuine(e: &Value, platform: &str, room: &str) -> bool {
    let s = |k: &str| e[k].as_str().unwrap_or("").to_string();
    s("tag") == format!("#{room}")
        && s("id") == event_id(e)
        && verify(&s("id"), &s("sig"), &s("author"))
        && verify(
            &format!("{}:{}", s("id"), e["server_ts"]),
            &s("server_sig"),
            platform,
        )
}
