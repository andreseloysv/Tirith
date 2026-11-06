//! The brain: small text jobs for a cheap, fast model of the Meta Model API (STUB: the misc agent replaces the
//! bodies, keeping every signature). Never on a hot path: callers use the cached answer or a plain fallback.

use crate::app::Shared;
use axum::Router;

/// One job ("room-summary", "room-needs-owner", "architecture", "epithet") on `input`: the model's answer,
/// cached by the hash of (job, input). `None` without an API key or when the call fails.
pub async fn ask(_app: &Shared, _job: &str, _input: &str) -> Option<String> {
    None
}

/// The project's architecture summary for lane briefs, for the current main commit; `None` until it exists
/// (asking starts its computation in the background).
pub fn architecture(_app: &Shared, _project: &str) -> Option<String> {
    None
}

pub fn start(_app: Shared) {}

pub fn routes() -> Router<Shared> {
    Router::new()
}

pub fn cli(_pos: &[&str], _flag: crate::client::Flag) -> Option<Result<(), String>> {
    None
}
