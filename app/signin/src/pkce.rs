//! PKCE (RFC 7636): a secret only this sign-in knows, and its S256 challenge for the authorization request.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use sha2::{Digest, Sha256};

/// A random URL-safe string of `bytes` bytes: the code verifier, or the state.
pub fn random(bytes: usize) -> String {
    B64.encode((0..bytes).map(|_| rand::random::<u8>()).collect::<Vec<_>>())
}

/// `BASE64URL(SHA256(verifier))`.
pub fn challenge(verifier: &str) -> String {
    B64.encode(Sha256::digest(verifier.as_bytes()))
}
