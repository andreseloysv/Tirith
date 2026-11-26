//! What a lane's run used, read from what its agent printed in the lane's log (and, for Copilot, its usage file).
use crate::turns::Usage;
use serde_json::Value;
use std::path::Path;

/// The usage of the run `log` ends: each line that can carry one (its words only, never a file the agent read).
pub fn from_log(log: &str) -> Usage {
    let mut usage = Usage::default();
    for line in crate::lanes::this_run(log)
        .lines()
        .filter(|l| l.starts_with('{'))
    {
        if line.contains("usage") || line.contains("\"result\"") || line.contains("turn.completed")
        {
            if let Ok(msg) = serde_json::from_str::<Value>(line) {
                usage.take(&msg);
            }
        }
    }
    usage
}

/// Copilot writes what its process used into a file as it ends: the session's totals so far.
pub fn from_file(usage: &mut Usage, file: &Path) {
    let data = std::fs::read(file)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
    if let Some(data) = data {
        usage.take(&serde_json::json!({"type": "copilot.usage", "data": data}));
    }
}
