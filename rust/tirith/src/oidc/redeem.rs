//! The provider's token endpoint: the code of a browser sign-in traded for the access token, with the PKCE
//! verifier and no client secret (the redirect address is a public client's in the provider).
use super::Settings;
use serde_json::Value;
use std::time::Duration;

/// The access token the provider gave for `code`, or why it did not.
pub(super) async fn token(
    s: &Settings,
    code: &str,
    verifier: &str,
    redirect: &str,
) -> Result<String, String> {
    let form = [
        ("grant_type", "authorization_code"),
        ("client_id", s.client_id.as_str()),
        ("code", code),
        ("redirect_uri", redirect),
        ("code_verifier", verifier),
        ("scope", &s.request_scope()),
    ];
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let sent = http.post(&s.token_url).form(&form).send().await;
    let resp = sent.map_err(|_| "the provider could not be reached".to_string())?;
    let ok = resp.status().is_success();
    let said: Value = resp.json().await.unwrap_or_default();
    if let Some(token) = said["access_token"].as_str().filter(|_| ok) {
        return Ok(token.to_string());
    }
    let why = said["error_description"]
        .as_str()
        .or(said["error"].as_str());
    let line = why
        .unwrap_or("no access token")
        .lines()
        .next()
        .unwrap_or("");
    Err(format!("the provider refused the code: {line}"))
}
