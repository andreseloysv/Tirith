//! The provider's signing keys: fetched from its JWKS address, kept, and fetched again when a token names a key id
//! that is not there (the provider rolled its keys), at most once a minute so a stream of bad tokens cannot make
//! this daemon hammer the provider.
use super::der::public_key;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const GAP: Duration = Duration::from_millis(if cfg!(test) { 400 } else { 60_000 });
type Keys = HashMap<String, Vec<u8>>;
static KEPT: Mutex<Option<HashMap<String, (Instant, Keys)>>> = Mutex::new(None);

fn kept(uri: &str, kid: &str) -> Result<Option<Vec<u8>>, ()> {
    let all = KEPT.lock().unwrap();
    let Some((at, keys)) = all.as_ref().and_then(|m| m.get(uri)) else {
        return Ok(None);
    };
    keys.get(kid).cloned().map(Some).ok_or(()).or_else(|_| {
        if at.elapsed() < GAP {
            Err(())
        } else {
            Ok(None)
        }
    })
}

/// The DER public key of `kid` at `uri`.
pub async fn key(uri: &str, kid: &str) -> Result<Vec<u8>, String> {
    match kept(uri, kid) {
        Ok(Some(k)) => return Ok(k),
        Err(()) => return Err(format!("unknown signing key {kid}")),
        Ok(None) => {}
    }
    let keys = fetch(uri).await?;
    let found = keys.get(kid).cloned();
    KEPT.lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(uri.into(), (Instant::now(), keys));
    found.ok_or_else(|| format!("unknown signing key {kid}"))
}

async fn fetch(uri: &str) -> Result<Keys, String> {
    let http = reqwest::Client::builder().timeout(Duration::from_secs(10));
    let sent = http
        .build()
        .map_err(|e| e.to_string())?
        .get(uri)
        .send()
        .await;
    let doc: Value = sent
        .map_err(|e| format!("keys: {e}"))?
        .json()
        .await
        .map_err(|e| format!("keys: {e}"))?;
    let part = |k: &Value, name: &str| B64.decode(k[name].as_str()?).ok();
    let rsa = doc["keys"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|k| k["kty"] == "RSA");
    Ok(rsa
        .filter_map(|k| {
            Some((
                k["kid"].as_str()?.to_string(),
                public_key(&part(k, "n")?, &part(k, "e")?),
            ))
        })
        .collect())
}
