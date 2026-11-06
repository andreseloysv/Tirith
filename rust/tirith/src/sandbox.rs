//! Network sandbox (STUB: the sandbox agent replaces this file's bodies, keeping every signature).
//!
//! Every process Tirith starts (lane agents, the manager, CEO reviews, merge-queue checks) runs inside a macOS
//! `sandbox-exec` profile that denies outbound network except loopback, with its proxy variables pointing at the
//! daemon's own HTTP/CONNECT proxy. The proxy authenticates the caller (`who` + token), applies the block list,
//! and records each new destination as an event. See the design notes in the module's brief.

use crate::app::Shared;
use axum::Router;
use serde_json::Value;

/// A process to start: program, arguments and extra environment.
pub struct Launch {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

impl Launch {
    pub fn command(&self) -> tokio::process::Command {
        let mut c = tokio::process::Command::new(&self.program);
        c.args(&self.args);
        for (k, v) in &self.env {
            c.env(k, v);
        }
        c
    }
}

/// Wraps `argv` for a process Tirith starts on behalf of `who` (a lane id, "manager", "ceo" or "merge:<lane>").
/// The real version returns `sandbox-exec -p <profile> argv…` plus the proxy environment and the PATH with the
/// tool wrappers first. The caller still sets its own cwd and its own env (TIRITH_*, PATH is set here when set).
pub fn wrap(_app: &Shared, _who: &str, argv: Vec<String>) -> Launch {
    let mut it = argv.into_iter();
    Launch { program: it.next().unwrap_or_default(), args: it.collect(), env: vec![] }
}

/// For agent sessions NOT started by Tirith (the owner's own sessions): the refusal when a tool call names a
/// blocked destination (a URL or host in a Bash command, a WebFetch url), else `None`.
pub fn check_command(_app: &Shared, _tool: &str, _input: &Value) -> Option<String> {
    None
}

/// Access-audit tags of one tool call: `(category, target)` with category one of "credential", "git_push",
/// "gh", "env" (target = variable names, never values). Recorded by the hook handler as `access` events.
pub fn tags(_tool: &str, _input: &Value) -> Vec<(&'static str, String)> {
    vec![]
}

/// Starts the proxy (and anything else the module needs) inside the daemon.
pub fn start(_app: Shared) {}

pub fn routes() -> Router<Shared> {
    Router::new()
}

/// `tirith net …` commands; `None` when `pos` is not one of this module's commands.
pub fn cli(_pos: &[&str], _flag: crate::client::Flag) -> Option<Result<(), String>> {
    None
}
