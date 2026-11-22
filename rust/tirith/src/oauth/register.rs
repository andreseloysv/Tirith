//! Dynamic client registration (RFC 7591), when the server offers it: Tirith as a public client of the daemon's own callback.
use serde_json::{json, Value};

/// A client of a server: its id, and its secret when it gave one.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct Client {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

/// Tirith registered at `endpoint` for `redirect`.
pub(super) async fn register(
    http: &reqwest::Client,
    endpoint: &str,
    redirect: &str,
) -> Result<Client, String> {
    let body = json!({
        "client_name": "Tirith",
        "redirect_uris": [redirect],
        "grant_types": ["authorization_code", "refresh_token"],
        "response_types": ["code"],
        "token_endpoint_auth_method": "none",
    });
    let resp = http.post(endpoint).json(&body).send().await;
    let resp = resp.map_err(|e| format!("cannot register at {endpoint}: {e}"))?;
    let status = resp.status();
    let said: Value = resp.json().await.unwrap_or_default();
    let id = said["client_id"].as_str().filter(|_| status.is_success());
    let id = id.ok_or_else(|| format!("{endpoint} refused the registration ({status})"))?;
    Ok(Client {
        id: id.into(),
        secret: said["client_secret"].as_str().map(String::from),
    })
}
