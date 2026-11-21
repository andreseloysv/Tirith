//! A lane running for long is reported to the manager once, with its last commands.
use super::logscan::last_commands;
use crate::app::{now_s, Lane, Shared};
use crate::{factory::Factory, manager};

/// Whether a running lane has gone `min` minutes since it started and was not reported yet (`min` 0: never).
pub(super) fn long_running_due(
    started_at: Option<&str>,
    now: u64,
    min: u64,
    reported: bool,
) -> bool {
    let start = started_at
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map_or(now, |t| t.timestamp().max(0) as u64);
    min > 0 && !reported && now.saturating_sub(start) >= min * 60
}
/// Tells the manager once that the running lane has gone past `limits.long_running_minutes`.
pub(super) fn report_long_running(app: &Shared, l: &Lane, text: &str, f: &Factory) {
    let key = format!("long_running:{}", l.id);
    if !(l.status == "running"
        && long_running_due(
            l.started_at.as_deref(),
            now_s(),
            f.limits.long_running_minutes,
            app.kv(&key).is_some(),
        ))
    {
        return;
    }
    app.set_kv(&key, Some("1"));
    let cmds = last_commands(text, 5)
        .iter()
        .map(|c| format!("- {c}"))
        .collect::<Vec<_>>()
        .join("\n");
    let text = format!("lane {} has been running for more than {} min. Check that it advances or whether it waits on something stuck. Its last commands:\n{cmds}", l.id, f.limits.long_running_minutes);
    manager::emit_event(app, &l.project, "lane.long", &text, None);
}
