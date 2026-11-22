//! One connector of an open gateway: its connection, local or remote, opened with the bot's secrets for it, and the tools it listed.
use super::{given::secrets, local::Local, remote::Remote, rpc};
use crate::app::Shared;
use crate::connectors::Connector;
use serde_json::{json, Value};
use std::time::Duration;

const LIMIT: Duration = Duration::from_secs(120);

enum Conn {
    Local(Box<tokio::sync::Mutex<Local>>),
    Remote(Remote),
}

pub struct Link {
    pub id: String,
    /// What its `tools/list` said, each tool as it came.
    pub catalog: Vec<Value>,
    conn: Conn,
}

impl Link {
    /// Connector `c` opened for bot `bot` (its page id), running as `who`: started or reached, greeted, its tools read.
    pub async fn open(app: &Shared, who: &str, bot: &str, c: &Connector) -> Result<Link, String> {
        let given = secrets(app, bot, &c.secrets).await?;
        let conn = if c.command.is_empty() {
            Conn::Remote(Remote::new(
                &crate::sandbox::proxy_url(app, who),
                &c.url,
                &given,
            )?)
        } else {
            let local = Local::start(app, who, &c.command, &given)?;
            Conn::Local(Box::new(tokio::sync::Mutex::new(local)))
        };
        drop(given);
        let mut link = Link {
            id: c.id.clone(),
            catalog: vec![],
            conn,
        };
        link.request("initialize", rpc::hello()).await?;
        link.notify("notifications/initialized").await?;
        let mut cursor = Value::Null;
        loop {
            let params = if cursor.is_null() {
                json!({})
            } else {
                json!({"cursor": cursor})
            };
            let page = link.request("tools/list", params).await?;
            link.catalog
                .extend(page["tools"].as_array().cloned().unwrap_or_default());
            cursor = page["nextCursor"].clone();
            if !cursor.is_string() {
                return Ok(link);
            }
        }
    }

    /// The names of the tools it listed.
    pub fn names(&self) -> Vec<String> {
        let names = self.catalog.iter().filter_map(|t| t["name"].as_str());
        names.map(String::from).collect()
    }

    pub async fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        match &self.conn {
            Conn::Local(l) => l.lock().await.request(method, params, LIMIT).await,
            Conn::Remote(r) => r.request(method, params).await,
        }
    }

    async fn notify(&self, method: &str) -> Result<(), String> {
        match &self.conn {
            Conn::Local(l) => l.lock().await.notify(method).await,
            Conn::Remote(r) => r.notify(method).await,
        }
    }
}
