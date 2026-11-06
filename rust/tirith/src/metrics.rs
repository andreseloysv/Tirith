//! Health metrics and the CEO (STUB: the metrics agent replaces the bodies, keeping every signature).
//!
//! Metrics are computed by code from the daemon's own data. A red metric is fixed by code when it is mechanical,
//! else reported to the manager; still red 15 minutes later, a one-shot independent review (the CEO) runs.

use crate::app::Shared;
use axum::Router;

pub fn start(_app: Shared) {}

pub fn routes() -> Router<Shared> {
    Router::new()
}

pub fn cli(_pos: &[&str], _flag: crate::client::Flag) -> Option<Result<(), String>> {
    None
}
