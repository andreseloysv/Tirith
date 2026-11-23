//! Codex runs a hook only once it is trusted: OpenBot's own are trusted through Codex's app-server, as its /hooks screen does. Nobody else's are.
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

struct Server {
    child: Child,
    to: ChildStdin,
    from: Receiver<Value>,
}

impl Server {
    fn start(program: &std::path::Path) -> Result<Server, String> {
        let mut child = Command::new(program)
            .arg("app-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("codex app-server: {e}"))?;
        let to = child.stdin.take().ok_or("codex app-server has no stdin")?;
        let out = child
            .stdout
            .take()
            .ok_or("codex app-server has no stdout")?;
        let (send, from) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                let _ = serde_json::from_str(&line).map(|v| send.send(v));
            }
        });
        Ok(Server { child, to, from })
    }

    fn tell(&mut self, msg: Value) -> Result<(), String> {
        writeln!(self.to, "{msg}")
            .and_then(|_| self.to.flush())
            .map_err(|e| format!("codex app-server: {e}"))
    }

    fn call(&mut self, id: i64, method: &str, params: Value) -> Result<Value, String> {
        self.tell(json!({"id": id, "method": method, "params": params}))?;
        loop {
            let msg = self.from.recv_timeout(Duration::from_secs(20));
            let msg = msg.map_err(|_| format!("codex app-server gave no answer to {method}"))?;
            if msg["id"] == id && msg.get("method").is_none() {
                let err = &msg["error"];
                return err
                    .is_null()
                    .then(|| msg["result"].clone())
                    .ok_or(format!("codex {method}: {err}"));
            }
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = (self.child.kill(), self.child.wait());
    }
}

/// Trusts OpenBot's hooks in Codex's settings; how many needed it.
pub fn trust(program: &std::path::Path) -> Result<usize, String> {
    let mut codex = Server::start(program)?;
    let me = json!({"name": "tirith", "title": "OpenBot", "version": env!("CARGO_PKG_VERSION")});
    codex.call(1, "initialize", json!({"clientInfo": me}))?;
    codex.tell(json!({"method": "initialized"}))?;
    let at = crate::config::home_dir().display().to_string();
    let found = codex.call(2, "hooks/list", json!({"cwds": [at]}))?;
    let mut state = serde_json::Map::new();
    let sets = found["data"].as_array().into_iter().flatten();
    for hook in sets.flat_map(|d| d["hooks"].as_array().into_iter().flatten()) {
        let mine = super::hook_lines::ours(hook) && hook["source"] == "user";
        if let (true, Some(key)) = (
            mine && hook["trustStatus"] != "trusted",
            hook["key"].as_str(),
        ) {
            state.insert(key.into(), json!({"trusted_hash": hook["currentHash"]}));
        }
    }
    let count = state.len();
    if count > 0 {
        let edit = json!({"keyPath": "hooks.state", "value": state, "mergeStrategy": "upsert"});
        codex.call(
            3,
            "config/batchWrite",
            json!({"edits": [edit], "reloadUserConfig": true}),
        )?;
    }
    Ok(count)
}
