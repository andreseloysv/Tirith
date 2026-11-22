//! The fallback's cipher: each bot's file has a key of its own, made from the daemon's, so one put in another bot's place does not open.
use crypto_secretbox::aead::{generic_array::GenericArray, Aead, KeyInit};
use crypto_secretbox::XSalsa20Poly1305;
use rand::RngCore;
use sha2::{Digest, Sha256};

const NONCE: usize = 24;

fn cipher(key: &[u8; 32], service: &str) -> XSalsa20Poly1305 {
    let mut made = Sha256::new();
    made.update(b"tirith bot secrets v1\0");
    made.update(key);
    made.update(service.as_bytes());
    XSalsa20Poly1305::new(GenericArray::from_slice(&made.finalize()))
}

pub(super) fn seal(key: &[u8; 32], service: &str, plain: &[u8]) -> Result<Vec<u8>, String> {
    let mut nonce = [0u8; NONCE];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let sealed = cipher(key, service)
        .encrypt(GenericArray::from_slice(&nonce), plain)
        .map_err(|_| "the secrets could not be sealed")?;
    Ok([&nonce[..], &sealed].concat())
}

pub(super) fn open(key: &[u8; 32], service: &str, sealed: &[u8]) -> Option<Vec<u8>> {
    let (nonce, body) = sealed.split_at_checked(NONCE)?;
    cipher(key, service)
        .decrypt(GenericArray::from_slice(nonce), body)
        .ok()
}
