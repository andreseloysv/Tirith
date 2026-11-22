//! Whether a replay's turn can run with the network closed.
use super::profile::enabled;
use crate::app::Shared;

/// Ok when the network sandbox is in force for the processes Tirith starts, so the proxy that refuses everything to a replay's caller (`lab::closed`) is the only way out; else why not (the replay would read the real network).
pub fn closes_network(app: &Shared) -> Result<(), String> {
    #[cfg(test)]
    if app.kv("test:network-closed").is_some() {
        return Ok(()); // the checks run their fake agents with no sandbox, and say so here
    }
    enabled(app)
}
