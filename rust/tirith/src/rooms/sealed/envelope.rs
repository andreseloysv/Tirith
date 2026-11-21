//! The sealed envelope: a message only its readers can open, signed inside by its sender.
use super::signed::{curve_public, inner_message, verify};
use super::{text::canonical, Me};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use crypto_box::aead::{generic_array::GenericArray, Aead, KeyInit};
use crypto_box::SalsaBox;
use crypto_secretbox::XSalsa20Poly1305;
use serde_json::{json, Value};
const ALG: &str = "x25519-xsalsa20-poly1305";
pub fn seal(me: &Me, text: &str, tag: &str, readers: &[String]) -> Value {
    let key: [u8; 32] = rand::random();
    let nonce: [u8; 24] = rand::random();
    let inner =
        canonical(&json!({"text": text, "sig": me.sign(&inner_message(&me.public, tag, text))}));
    let mine = me.curve_secret();
    let mut pubs: Vec<&String> = readers.iter().chain([&me.public]).collect();
    pubs.sort();
    pubs.dedup();
    let wrap_for = |p: &String| {
        let wrap: [u8; 24] = rand::random();
        let boxed = SalsaBox::new(&curve_public(p)?, &mine)
            .encrypt(GenericArray::from_slice(&wrap), &key[..])
            .ok()?;
        Some(json!({"pub": p, "wrapped_key": B64.encode([&wrap[..], &boxed].concat())}))
    };
    let wrapped: Vec<Value> = pubs.into_iter().filter_map(wrap_for).collect();
    let cipher = XSalsa20Poly1305::new(GenericArray::from_slice(&key))
        .encrypt(GenericArray::from_slice(&nonce), inner.as_bytes())
        .unwrap_or_default();
    json!({"v": 1, "alg": ALG, "sender_pub": me.public, "recipients": wrapped, "nonce": B64.encode(nonce), "ciphertext": B64.encode(cipher)})
}
/// The words of an event sealed for me, or `None`: not for me, forged, or not the sender's own.
pub fn unseal(me: &Me, e: &Value) -> Option<String> {
    let body = &e["body"];
    let author = e["author"].as_str()?;
    (body["sender_pub"].as_str() == Some(author) && body["alg"] == ALG).then_some(())?;
    let mine = body["recipients"]
        .as_array()?
        .iter()
        .find(|r| r["pub"].as_str() == Some(&me.public))?;
    let wrapped = B64.decode(mine["wrapped_key"].as_str()?).ok()?;
    (wrapped.len() >= 24 + 16).then_some(())?;
    let key = SalsaBox::new(&curve_public(author)?, &me.curve_secret())
        .decrypt(GenericArray::from_slice(&wrapped[..24]), &wrapped[24..])
        .ok()?;
    let nonce = B64
        .decode(body["nonce"].as_str()?)
        .ok()
        .filter(|n| n.len() == 24)?;
    let cipher = B64.decode(body["ciphertext"].as_str()?).ok()?;
    let plain = XSalsa20Poly1305::new_from_slice(&key)
        .ok()?
        .decrypt(GenericArray::from_slice(&nonce), &cipher[..])
        .ok()?;
    let inner: Value = serde_json::from_slice(&plain).ok()?;
    let text = inner["text"].as_str()?;
    verify(
        &inner_message(author, e["tag"].as_str()?, text),
        inner["sig"].as_str()?,
        author,
    )
    .then(|| text.to_string())
}
