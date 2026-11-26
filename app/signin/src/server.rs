//! The team server's side: what it says about its provider, and the handing over of the access token.
use serde_json::Value;

/// What the server tells the app about its provider.
pub struct Provider {
    pub client_id: String,
    pub scope: String,
    pub authorize: String,
    pub token: String,
}

/// An https address, or a loopback one (a tunnel, a provider of the checks): nothing else carries a token.
fn safe(url: &str) -> Result<(), String> {
    let loopback = ["http://127.0.0.1", "http://localhost", "http://[::1]"];
    let ok = url.starts_with("https://") || loopback.iter().any(|l| url.starts_with(l));
    ok.then_some(())
        .ok_or_else(|| format!("{url} is not https"))
}

fn host(url: &str) -> &str {
    url.split("://")
        .nth(1)
        .unwrap_or("")
        .split(['/', '?'])
        .next()
        .unwrap_or("")
}

pub fn provider(server: &str, http: &reqwest::blocking::Client) -> Result<Provider, String> {
    let said = http
        .get(format!("{server}/api/auth/oidc"))
        .send()
        .map_err(|e| format!("{server}: {e}"))?;
    if !said.status().is_success() {
        return Err(format!("{server} has no team sign-in ({})", said.status()));
    }
    let v: Value = said.json().map_err(|e| e.to_string())?;
    let text = |p: &[&str]| {
        p.iter()
            .try_fold(&v, |v, k| v.get(k))
            .and_then(Value::as_str)
            .map(String::from)
    };
    let get =
        |p: &[&str]| text(p).ok_or_else(|| "the server's sign-in answer is incomplete".to_string());
    let p = Provider {
        client_id: get(&["client_id"])?,
        scope: get(&["scope"])?,
        authorize: get(&["endpoints", "authorization_endpoint"])?,
        token: get(&["endpoints", "token_endpoint"])?,
    };
    let issuer = get(&["issuer"])?;
    [&p.authorize, &p.token].iter().try_for_each(|u| safe(u))?;
    (host(&p.authorize) == host(&issuer) && host(&p.token) == host(&issuer))
        .then_some(p)
        .ok_or_else(|| "the server's endpoints are not its issuer's".to_string())
}

/// Hands the access token to the server: its session, as `(name, role, link)`.
pub fn hand_over(
    server: &str,
    token: &str,
    http: &reqwest::blocking::Client,
) -> Result<(String, String, String), String> {
    let said = http
        .post(format!("{server}/api/auth/oidc"))
        .bearer_auth(token)
        .send()
        .map_err(|e| e.to_string())?;
    let status = said.status();
    let v: Value = said.json().map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(v["error"]
            .as_str()
            .map_or_else(|| format!("refused ({status})"), String::from));
    }
    let get = |k: &str| v[k].as_str().unwrap_or("").to_string();
    Ok((get("name"), get("role"), get("link")))
}
