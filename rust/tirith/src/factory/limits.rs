//! The factory's `[limits]`: load, time and size limits the daemon runs under.
use serde::Deserialize;

#[derive(Deserialize, Clone, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    pub max_load: f64,
    pub inactivity_minutes: u64,
    pub long_running_minutes: u64, // a lane running this long is reported once to its manager (0 turns it off)
    pub stage_timeout_minutes: u64,
    pub merge_train: usize,
    pub merge_backlog: usize,
    pub low_disk_gb: f64,
    pub build_idle_minutes: u64, // minutes with no lane, stage or manager working before a project's build cache and merge folder go
    pub plan_commit_seconds: u64,
    pub batch_seconds: u64,
    pub agent_messages_in_a_row: u32,
    pub routine_minutes: u64, // a routine's run that waits or works longer failed (0 is 60)
    pub max_parallel: Option<usize>, // a cap on lanes at once; none unless config.toml sets one
}

/// The `[limits]` section of a merged factory table, strictly read.
pub(super) fn read(raw: &toml::Table) -> Result<Limits, String> {
    Ok(raw
        .get("limits")
        .cloned()
        .map(|v| v.try_into())
        .transpose()
        .map_err(|e| format!("factory: {e}"))?
        .unwrap_or_default())
}
