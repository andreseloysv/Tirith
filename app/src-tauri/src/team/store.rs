//! The team servers the person added, and which one the window shows: `team-servers.json` in the app's own config
//! folder. Addresses and the last name and role seen: nothing the provider issued, no token, no refresh token.
use serde_json::{json, Value};
use std::path::PathBuf;

pub struct Store(pub PathBuf);

impl Store {
    pub fn read(&self) -> Value {
        let text = std::fs::read_to_string(&self.0).unwrap_or_default();
        serde_json::from_str(&text).unwrap_or_else(|_| json!({"servers": [], "active": null, "people": {}}))
    }

    fn write(&self, v: &Value) -> Result<(), String> {
        std::fs::create_dir_all(self.0.parent().ok_or("no folder")?).map_err(|e| e.to_string())?;
        std::fs::write(&self.0, v.to_string()).map_err(|e| e.to_string())
    }

    /// The address that is kept for what was typed: `https://host[:port]` or `http://127.0.0.1[:port]`, no path.
    pub fn address(typed: &str) -> Result<String, String> {
        let typed = typed.trim().trim_end_matches('/');
        let (scheme, rest) = typed.split_once("://").ok_or("an address like https://neobot.team.example")?;
        let host = rest.split(['/', '?', '#']).next().unwrap_or("");
        let name = host.rsplit_once(':').map_or(host, |(n, _)| n);
        let local = matches!(name, "127.0.0.1" | "localhost" | "[::1]");
        let ok = !host.is_empty() && rest == host && (scheme == "https" || (scheme == "http" && local));
        ok.then(|| format!("{scheme}://{host}")).ok_or_else(|| "use https, or http only for 127.0.0.1 (a tunnel); no path".into())
    }

    pub fn add(&self, typed: &str) -> Result<(), String> {
        let (url, mut v) = (Self::address(typed)?, self.read());
        let list = v["servers"].as_array_mut().ok_or("damaged file")?;
        (!list.iter().any(|s| *s == url)).then(|| list.push(json!(url)));
        self.write(&v)
    }

    pub fn remove(&self, url: &str) -> Result<(), String> {
        let mut v = self.read();
        v["servers"].as_array_mut().map(|l| l.retain(|s| s != url));
        v["people"].as_object_mut().map(|p| p.remove(url));
        (v["active"] == url).then(|| v["active"] = Value::Null);
        self.write(&v)
    }

    /// Shows `url` (a server) or this Mac (none).
    pub fn activate(&self, url: Option<&str>) -> Result<(), String> {
        let mut v = self.read();
        v["active"] = url.map_or(Value::Null, |u| json!(u));
        self.write(&v)
    }

    /// The name and role the server last signed this person in as, for the window's title.
    pub fn signed(&self, url: &str, who: Option<(&str, &str)>) -> Result<(), String> {
        let mut v = self.read();
        let what = who.map_or(Value::Null, |(n, r)| json!({"name": n, "role": r}));
        v["people"][url] = what;
        self.write(&v)
    }
}
