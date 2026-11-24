//! The gateway process's way to the daemon: its port and the caller token of the run that started it, nothing else.
use serde_json::Value;
use std::time::Duration;

pub struct Door {
    pub port: u16,
    pub token: Vec<(&'static str, String)>,
}

impl Door {
    /// This process's: the daemon of its NeoBot home, and its run's token (never the owner's: a run's process has none to read).
    pub fn here() -> Door {
        let port = crate::config::Config::load_at(&crate::config::tirith_home()).port;
        Door {
            port,
            token: crate::client::hook_caller(),
        }
    }

    /// One request; its JSON answer, or what the daemon said.
    pub fn ask(&self, method: &str, path: &str, body: &Value) -> Result<Value, String> {
        let body = if body.is_null() {
            String::new()
        } else {
            body.to_string()
        };
        let wait = Duration::from_secs(300); // a connector's call may take long
        let said = crate::client::call_on(self.port, (method, path), body, &self.token, wait)?;
        serde_json::from_str(&said).map_err(|e| e.to_string())
    }
}
