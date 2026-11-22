//! OAuth discovery of a remote MCP server: its protected-resource metadata (RFC 9728, path-suffixed first), then its authorization server's metadata (RFC 8414, then OpenID's); a server with neither has the default endpoints at its origin.
use reqwest::Url;
use serde_json::Value;

/// Where a server signs in.
#[derive(Debug, Default)]
pub(super) struct Server {
    pub authorize: String,
    pub token: String,
    pub register: Option<String>,
    pub scopes: Vec<String>,
}

/// The well-known addresses of `name` for `url`: with its path appended, then at the origin.
fn known(url: &Url, name: &str) -> Vec<String> {
    let origin = url.origin().ascii_serialization();
    let path = url.path().trim_end_matches('/');
    let mut all = vec![format!("{origin}/.well-known/{name}{path}")];
    if !path.is_empty() {
        all.push(format!("{origin}/.well-known/{name}"));
    }
    all
}

/// The first of `urls` that answers a JSON object.
async fn first(http: &reqwest::Client, urls: &[String]) -> Option<Value> {
    for url in urls {
        let Ok(resp) = http
            .get(url)
            .header("accept", "application/json")
            .send()
            .await
        else {
            continue;
        };
        let found = match resp.status().is_success() {
            true => resp.json::<Value>().await.ok(),
            false => None,
        };
        if let Some(v) = found.filter(Value::is_object) {
            return Some(v);
        }
    }
    None
}

fn texts(v: &Value) -> Vec<String> {
    let all = v.as_array().into_iter().flatten();
    all.filter_map(|s| s.as_str().map(String::from)).collect()
}

/// The sign-in of the MCP server at `mcp`.
pub(super) async fn discover(http: &reqwest::Client, mcp: &str) -> Result<Server, String> {
    let url = Url::parse(mcp).map_err(|e| format!("{mcp}: {e}"))?;
    let resource = first(http, &known(&url, "oauth-protected-resource")).await;
    let resource = resource.unwrap_or_default();
    let issuer = resource["authorization_servers"][0].as_str();
    let issuer = issuer.map_or_else(|| url.origin().ascii_serialization(), String::from);
    let at = Url::parse(&issuer).map_err(|e| format!("{issuer}: {e}"))?;
    let mut tried = known(&at, "oauth-authorization-server");
    tried.extend(known(&at, "openid-configuration"));
    let meta = first(http, &tried).await;
    let base = at.origin().ascii_serialization();
    let field = |m: &Value, k: &str, default: &str| {
        m[k].as_str()
            .map_or(format!("{base}{default}"), String::from)
    };
    let meta = meta.unwrap_or_else(
        || serde_json::json!({"registration_endpoint": format!("{base}/register")}),
    );
    let methods = texts(&meta["code_challenge_methods_supported"]);
    if !methods.is_empty() && !methods.iter().any(|m| m == "S256") {
        return Err(format!("{issuer} offers no PKCE with S256"));
    }
    Ok(Server {
        authorize: field(&meta, "authorization_endpoint", "/authorize"),
        token: field(&meta, "token_endpoint", "/token"),
        register: meta["registration_endpoint"].as_str().map(String::from),
        scopes: texts(&resource["scopes_supported"]),
    })
}
