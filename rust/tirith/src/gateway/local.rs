//! A local connector: its argv started in the bot's sandbox (`sandbox::wrap`, as the bot), its secrets in that process's environment alone, one JSON-RPC message per line each way.
use super::rpc;
use crate::app::Shared;
use serde_json::Value;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout};

pub struct Local {
    child: Child,
    stdin: ChildStdin,
    lines: Lines<BufReader<ChildStdout>>,
    next: u64,
}

impl Local {
    /// The server of `argv`, started for `who` with `secrets` (name, value) in its environment.
    pub fn start(
        app: &Shared,
        who: &str,
        argv: &[String],
        secrets: &[(String, String)],
    ) -> Result<Local, String> {
        let launch = crate::sandbox::wrap(app, who, argv.to_vec());
        let mut cmd = launch.command(app);
        cmd.envs(secrets.iter().map(|(k, v)| (k, v)))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = cmd.spawn().map_err(|e| format!("cannot start it: {e}"))?;
        let (stdin, stdout) = (child.stdin.take(), child.stdout.take());
        let (Some(stdin), Some(stdout)) = (stdin, stdout) else {
            return Err("it has no stdin or stdout".into());
        };
        let lines = BufReader::new(stdout).lines();
        Ok(Local {
            child,
            stdin,
            lines,
            next: 0,
        })
    }

    async fn send(&mut self, msg: &Value) -> Result<(), String> {
        let line = format!("{msg}\n");
        let wrote = self.stdin.write_all(line.as_bytes()).await;
        wrote.map_err(|e| format!("it stopped reading: {e}"))?;
        self.stdin.flush().await.map_err(|e| e.to_string())
    }

    /// The answer to `method`, within `limit`; what it says in between (notifications, its own requests) is passed over.
    pub async fn request(
        &mut self,
        method: &str,
        params: Value,
        limit: Duration,
    ) -> Result<Value, String> {
        self.next += 1;
        let id = self.next;
        self.send(&rpc::request(id, method, params)).await?;
        let read = async {
            while let Some(line) = self.lines.next_line().await.map_err(|e| e.to_string())? {
                let msg: Value = serde_json::from_str(&line).unwrap_or_default();
                if rpc::answers(&msg, id) {
                    return rpc::answer(msg);
                }
            }
            Err("it ended".to_string())
        };
        tokio::time::timeout(limit, read)
            .await
            .map_err(|_| format!("no answer to {method} within {} s", limit.as_secs()))?
    }

    pub async fn notify(&mut self, method: &str) -> Result<(), String> {
        self.send(&rpc::notification(method)).await
    }
}

/// Whatever it started goes with it.
impl Drop for Local {
    fn drop(&mut self) {
        if let Some(pid) = self.child.id() {
            crate::app::kill_group(pid, crate::app::KILL);
        }
    }
}
