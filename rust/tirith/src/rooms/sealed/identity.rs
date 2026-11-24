//! This NeoBot's identity in the rooms: its ed25519 key in `<home>/am/<name>.json`.
use super::text::{hex, key_id, unhex};
use crate::os::Mode;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::{json, Value};
use sha2::{Digest, Sha512};
use std::{io::Write, path::Path};
/// This NeoBot's identity in the rooms. The secret never leaves the `sealed` module (no Debug, no Serialize).
pub struct Me {
    pub name: String,
    pub public: String,
    pub id: String,
    pub(super) key: SigningKey,
}
/// Loads `<home>/am/<name>.json` (am.mjs's key file format), making it on first use.
pub fn identity(home: &Path, name: &str) -> Result<Me, String> {
    let dir = home.join("am");
    let path = dir.join(format!("{name}.json"));
    if !path.exists() {
        make_key_file(&dir, &path, name)?;
    }
    let v: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let bytes = unhex(v["secretKey"].as_str().unwrap_or(""))
        .filter(|b| b.len() == 64)
        .ok_or("the room key file is damaged")?;
    let key = SigningKey::from_keypair_bytes(&bytes.try_into().unwrap_or([0; 64]))
        .map_err(|_| "the room key file is damaged")?;
    let public = hex(key.verifying_key().as_bytes());
    Ok(Me {
        name: name.to_string(),
        id: key_id(&public),
        public,
        key,
    })
}
/// A new key in a new file (dir 0700, file 0600).
fn make_key_file(dir: &Path, path: &Path, name: &str) -> Result<(), String> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .with_mode(0o700)
        .create(dir)
        .map_err(|e| e.to_string())?;
    let secret = hex(&SigningKey::from_bytes(&rand::random()).to_keypair_bytes());
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .with_mode(0o600)
        .open(path)
        .map_err(|e| e.to_string())?;
    f.write_all(
        json!({"name": name, "secretKey": secret})
            .to_string()
            .as_bytes(),
    )
    .map_err(|e| e.to_string())
}
impl Me {
    pub fn sign(&self, message: &str) -> String {
        hex(&self.key.sign(message.as_bytes()).to_bytes())
    }
    pub(super) fn curve_secret(&self) -> crypto_box::SecretKey {
        crypto_box::SecretKey::from(
            <[u8; 32]>::try_from(&Sha512::digest(self.key.to_bytes())[..32]).unwrap(),
        )
    }
}
