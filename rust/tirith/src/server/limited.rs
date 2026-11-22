//! A command of a Codex under its caller's limit (read-only, workspace-write) reaches the daemon's port for the rules' hook alone: Tirith changes nothing for it, or the limit would be no limit.
use axum::http::Method;

pub(super) const WHY: &str = "a command under Codex's sandbox limit changes nothing through Tirith: say in your answer what is needed";

/// Where the kernel is asked whether such a command calls: whatever changes something, and is no hook.
pub(super) fn changes(method: &Method, path: &str) -> bool {
    let reads = [Method::GET, Method::HEAD, Method::OPTIONS].contains(method);
    cfg!(target_os = "macos") && !reads && !path.starts_with("/hook/")
}
