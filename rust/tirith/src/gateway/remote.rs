//! A remote connector: MCP's streamable HTTP (each message a POST, the answer JSON or an event stream), always through Tirith's proxy with the bot's credentials, so the proxy counts it as the bot's; its secrets in the auth headers alone: the first `Authorization: Bearer`, any other a header of its own name (`_` as `-`).
use super::{auth::auth, rpc};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

const SESSION: &str = "mcp-session-id";

pub struct Remote {
    client: reqwest::Client,
    url: String,
    session: Mutex<Option<String>>,
    next: AtomicU64,
}

impl Remote {
    /// The server at `url`, reached through `proxy` (the proxy's address with the bot's credentials).
    pub fn new(proxy: &str, url: &str, secrets: &[(String, String)]) -> Result<Remote, String> {
        let client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(proxy).map_err(|e| e.to_string())?)
            .default_headers(auth(secrets)?)
            .pool_max_idle_per_host(0)
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| e.to_string())?;
        let (session, next) = (Mutex::new(None), AtomicU64::new(0));
        Ok(Remote {
            client,
            url: url.into(),
            session,
            next,
        })
    }

    async fn post(&self, msg: &Value) -> Result<(String, String), String> {
        let mut req = self.client.post(&self.url).json(msg);
        req = req.header("accept", "application/json, text/event-stream");
        if let Some(s) = self.session.lock().unwrap().clone() {
            req = req.header(SESSION, s);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("cannot reach it: {e}"))?;
        let status = resp.status();
        let header = |k: &str| {
            resp.headers()
                .get(k)
                .and_then(|v| v.to_str().ok())
                .map(String::from)
        };
        if let Some(s) = header(SESSION) {
            *self.session.lock().unwrap() = Some(s);
        }
        let kind = header("content-type").unwrap_or_default();
        let body = resp.text().await.map_err(|e| e.to_string())?;
        let Some(()) = status.is_success().then_some(()) else {
            return Err(format!("it answered {status}"));
        };
        Ok((kind, body))
    }

    pub async fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        let (kind, body) = self.post(&rpc::request(id, method, params)).await?;
        let msgs: Vec<Value> = if kind.starts_with("text/event-stream") {
            body.lines()
                .filter_map(|l| l.strip_prefix("data:"))
                .filter_map(|d| serde_json::from_str(d.trim()).ok())
                .collect()
        } else {
            serde_json::from_str(&body).into_iter().collect()
        };
        let found = msgs.into_iter().find(|m| rpc::answers(m, id));
        rpc::answer(found.ok_or(format!("no answer to {method}"))?)
    }

    pub async fn notify(&self, method: &str) -> Result<(), String> {
        self.post(&rpc::notification(method)).await.map(drop)
    }
}
