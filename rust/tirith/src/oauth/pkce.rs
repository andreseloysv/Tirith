//! PKCE (S256) and the state: random, base64url without padding.
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};

fn random(n: usize) -> String {
    let mut bytes = vec![0u8; n];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// A code verifier of 43 characters.
pub(super) fn verifier() -> String {
    random(32)
}

/// The S256 challenge of `verifier`.
pub(super) fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// A state, or a ticket: unguessable and used once.
pub(super) fn state() -> String {
    random(24)
}
