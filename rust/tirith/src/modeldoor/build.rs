//! The request to the provider: what of the caller's goes on, and how the real key signs it.
use super::spec::{Provider, Style};
use axum::http::HeaderName;
use std::{sync::LazyLock, time::Duration};

pub(super) static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    let client = reqwest::Client::builder().connect_timeout(Duration::from_secs(15));
    client
        .read_timeout(Duration::from_secs(900))
        .build()
        .unwrap_or_default()
});

/// What the caller said that goes on to the provider: not who it is, how it signs, nor how the answer is packed.
pub(super) fn passes(name: &HeaderName) -> bool {
    ![
        "host",
        "authorization",
        "x-api-key",
        "api-key",
        "x-tirith-token",
        "cookie",
        "content-length",
        "connection",
        "accept-encoding",
        "transfer-encoding",
        "proxy-authorization",
    ]
    .contains(&name.as_str())
}

pub(super) fn key_header(p: &Provider) -> (&'static str, String) {
    match p.style {
        Style::Anthropic => ("x-api-key", p.key.clone()),
        Style::OpenAi => ("authorization", format!("Bearer {}", p.key)),
        Style::Azure => ("api-key", p.key.clone()),
    }
}

/// The model the request names, else the Azure deployment of its path.
pub(super) fn model(path: &str, body: &[u8]) -> String {
    let named = serde_json::from_slice::<serde_json::Value>(body).ok();
    let named = named.and_then(|v| v["model"].as_str().map(String::from));
    let deployment = path
        .strip_prefix("/openai/deployments/")
        .and_then(|r| r.split('/').next());
    named
        .or_else(|| deployment.map(String::from))
        .unwrap_or_default()
}
