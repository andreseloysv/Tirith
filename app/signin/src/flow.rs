//! The whole sign-in, in order.
use super::{loopback::Redirect, pkce, server};
use std::time::Duration;

/// The person as the server knows them now, and the server's one-use link that signs the window in.
pub struct Session {
    pub name: String,
    pub role: String,
    pub link: String,
}

/// Signs in to `server` (its address without a trailing slash). `browse` is given the provider's address to open in the browser.
pub fn sign_in(
    server: &str,
    browse: &dyn Fn(&str) -> Result<(), String>,
    wait: Duration,
) -> Result<Session, String> {
    let http = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;
    let p = server::provider(server, &http)?;
    let (verifier, state) = (pkce::random(48), pkce::random(24));
    let redirect = Redirect::open()?;
    let mut url = reqwest::Url::parse(&p.authorize).map_err(|e| e.to_string())?;
    url.query_pairs_mut().extend_pairs([
        ("client_id", p.client_id.as_str()),
        ("response_type", "code"),
        ("redirect_uri", &redirect.uri()),
        ("response_mode", "query"),
        ("scope", &p.scope),
        ("state", &state),
        ("code_challenge", &pkce::challenge(&verifier)),
        ("code_challenge_method", "S256"),
        ("prompt", "select_account"),
    ]);
    browse(url.as_str())?;
    let code = redirect.code(&state, wait)?;
    let form = [
        ("grant_type", "authorization_code"),
        ("client_id", &p.client_id),
        ("code", &code),
        ("redirect_uri", &redirect.uri()),
        ("code_verifier", &verifier),
        ("scope", &p.scope),
    ];
    let said = http
        .post(&p.token)
        .form(&form)
        .send()
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value = said.json().map_err(|e| e.to_string())?;
    let token = v["access_token"].as_str().ok_or_else(|| {
        format!(
            "the provider gave no token: {}",
            v["error_description"].as_str().unwrap_or("?")
        )
    })?;
    let (name, role, link) = server::hand_over(server, token, &http)?;
    Ok(Session { name, role, link })
}
