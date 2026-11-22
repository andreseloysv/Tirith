//! The token endpoint: a code exchanged (with its PKCE verifier) and a refresh, each sending the resource it is for (RFC 8707).
use super::register::Client;
use serde_json::Value;

/// What a token endpoint gave.
pub(super) struct Tokens {
    pub access: String,
    pub refresh: Option<String>,
    /// Seconds it lives, when the server said.
    pub lives: Option<u64>,
}

/// Where a sign-in is renewed: kept beside its tokens.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct Grant {
    pub token_url: String,
    pub client: Client,
    pub resource: String,
    /// Unix seconds its access ends; 0 when the server did not say.
    pub expires_at: u64,
}

/// `form` posted to the grant's token endpoint, the client's id (and secret) added.
async fn post(grant: &Grant, mut form: Vec<(&str, String)>) -> Result<Tokens, String> {
    form.push(("client_id", grant.client.id.clone()));
    form.push(("resource", grant.resource.clone()));
    if let Some(secret) = &grant.client.secret {
        form.push(("client_secret", secret.clone()));
    }
    let resp = super::http()?
        .post(&grant.token_url)
        .header("accept", "application/json")
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("cannot reach the token endpoint: {e}"))?;
    let status = resp.status();
    let said: Value = resp.json().await.unwrap_or_default();
    let access = said["access_token"]
        .as_str()
        .filter(|_| status.is_success());
    let Some(access) = access else {
        let why = said["error"].as_str().unwrap_or("no access token");
        return Err(format!("the token endpoint answered {status}: {why}"));
    };
    Ok(Tokens {
        access: access.into(),
        refresh: said["refresh_token"].as_str().map(String::from),
        lives: said["expires_in"].as_u64(),
    })
}

/// The code of a sign-in exchanged for tokens.
pub(super) async fn exchange(
    grant: &Grant,
    code: &str,
    verifier: &str,
    redirect: &str,
) -> Result<Tokens, String> {
    let form = vec![
        ("grant_type", "authorization_code".into()),
        ("code", code.into()),
        ("code_verifier", verifier.into()),
        ("redirect_uri", redirect.into()),
    ];
    post(grant, form).await
}

/// New tokens for a refresh token.
pub(super) async fn refresh(grant: &Grant, refresh: &str) -> Result<Tokens, String> {
    let form = vec![
        ("grant_type", "refresh_token".into()),
        ("refresh_token", refresh.into()),
    ];
    post(grant, form).await
}
