//! A remote connector's secrets as its auth headers: the first `Authorization: Bearer <value>`, any other a header of its own name with `_` as `-`, each marked sensitive so no log of the client shows it.
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

/// The headers the secrets go in, each marked sensitive.
pub fn auth(secrets: &[(String, String)]) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    for (i, (name, value)) in secrets.iter().enumerate() {
        let (key, value) = match i {
            0 => (
                HeaderName::from_static("authorization"),
                format!("Bearer {value}"),
            ),
            _ => (
                HeaderName::try_from(name.replace('_', "-")).map_err(|e| e.to_string())?,
                value.clone(),
            ),
        };
        let mut value = HeaderValue::try_from(value)
            .map_err(|_| format!("secret {name} is no header value"))?;
        value.set_sensitive(true);
        headers.insert(key, value);
    }
    Ok(headers)
}
