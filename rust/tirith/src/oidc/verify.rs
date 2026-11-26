//! Checking an access token: RS256 against the provider's keys, then what it says (issuer, audience, times, tenant,
//! delegated scope, app role). A token that fails any one is refused, and the reason is told to its holder.
use super::{jwks, Level, Settings};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use serde_json::Value;

/// Who the token is, once it is proven.
pub struct Claims {
    pub oid: String,
    pub name: String,
    pub username: String,
    pub level: Level,
}

fn part(text: &str) -> Result<Vec<u8>, String> {
    B64.decode(text).map_err(|_| "not a token".to_string())
}

fn json(text: &str) -> Result<Value, String> {
    serde_json::from_slice(&part(text)?).map_err(|_| "not a token".to_string())
}

/// The token's claims, when its signature is the provider's and it says what this NeoBot accepts.
pub async fn check(s: &Settings, token: &str) -> Result<Claims, String> {
    let parts: Vec<&str> = token.trim().split('.').collect();
    let [head, body, sig] = parts.as_slice() else {
        return Err("not a token".into());
    };
    let header = json(head)?;
    if header["alg"] != "RS256" {
        return Err("the token is not signed with RS256".into());
    }
    let key = jwks::key(&s.jwks_uri, header["kid"].as_str().unwrap_or("")).await?;
    let signed = format!("{head}.{body}");
    let rsa = &ring::signature::RSA_PKCS1_2048_8192_SHA256;
    ring::signature::UnparsedPublicKey::new(rsa, key)
        .verify(signed.as_bytes(), &part(sig)?)
        .map_err(|_| "the token's signature is not the provider's".to_string())?;
    claims(s, &json(body)?, crate::app::now_s())
}

fn text(c: &Value, k: &str) -> String {
    c[k].as_str().unwrap_or("").to_string()
}

/// What a proven token says, against this NeoBot's settings, at `now` seconds since the epoch.
pub(super) fn claims(s: &Settings, c: &Value, now: u64) -> Result<Claims, String> {
    if c["iss"] != s.issuer.as_str() {
        return Err("wrong issuer".into());
    }
    let auds = c["aud"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| vec![c["aud"].clone()]);
    if !auds
        .iter()
        .any(|a| s.audience.iter().any(|w| a == w.as_str()))
    {
        return Err("the token is for another audience".into());
    }
    let (exp, nbf) = (c["exp"].as_u64(), c["nbf"].as_u64().unwrap_or(0));
    if exp.is_none_or(|e| e + s.leeway < now) {
        return Err("the token expired".into());
    }
    if nbf > now + s.leeway {
        return Err("the token is not valid yet".into());
    }
    if c["tid"] != s.tenant.as_str() {
        return Err("the token is from another tenant".into());
    }
    let scopes = text(c, "scp");
    if !s.scope.is_empty() && !scopes.split(' ').any(|x| x == s.scope) {
        return Err(format!("the token lacks the scope {}", s.scope));
    }
    let held: Vec<String> = c["roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r.as_str().map(String::from))
        .collect();
    let level = s
        .level(&held)
        .ok_or("no NeoBot role is assigned to this account")?;
    let oid = text(c, "oid");
    if oid.is_empty() {
        return Err("the token names no user".into());
    }
    let username = text(c, "preferred_username");
    let name = Some(text(c, "name"))
        .filter(|n| !n.is_empty())
        .unwrap_or(username.clone());
    Ok(Claims {
        oid,
        name,
        username,
        level,
    })
}
