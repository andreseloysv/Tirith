//! The proxy passwords: each derives from a random secret kept in the daemon's database.
use crate::{
    app::Shared,
    rooms::sealed::{hex, sha_hex},
};
use std::sync::OnceLock;

/// The secret every proxy token derives from.
static SECRET: OnceLock<String> = OnceLock::new();

/// Loads the secret from the database, making one on the first start.
pub fn load_secret(app: &Shared) {
    let secret = app.kv("net_secret").unwrap_or_else(|| {
        let s = hex(&rand::random::<[u8; 32]>());
        app.set_kv("net_secret", Some(&s));
        s
    });
    let _ = SECRET.set(secret);
}

/// The proxy password of `who`: derived from the secret, so processes started before a daemon restart keep their access.
pub fn token(who: &str) -> String {
    sha_hex(format!("{}\n{who}", SECRET.get().map(String::as_str).unwrap_or("")).as_bytes())[..32]
        .to_string()
}
