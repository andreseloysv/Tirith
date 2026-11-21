//! A stage's time limit: the manager's, else the one learned from past timeouts, else the default.
use crate::app::{now_s, Shared};
pub(super) const MANAGER_LIMIT_SECS: u64 = 24 * 3600;
/// The limit in minutes for a stage: the manager's (within 24 h), else the one learned from past timeouts, else the default. Each project and stage has its own.
pub(super) fn limit_from(
    get: impl Fn(&str) -> Option<String>,
    project: &str,
    stage: &str,
    default: u64,
) -> u64 {
    let set = get(&format!("stage_limit:{project}:{stage}")).and_then(|v| {
        v.split_once('@').and_then(|(m, at)| {
            (now_s().saturating_sub(at.parse().ok()?) < MANAGER_LIMIT_SECS)
                .then(|| m.parse().ok())
                .flatten()
        })
    });
    set.or_else(|| get(&format!("stage_learned:{project}:{stage}")).and_then(|v| v.parse().ok()))
        .unwrap_or(default)
}
/// The limit in minutes of a stage of a project, with the factory's default.
pub(super) fn stage_minutes(app: &Shared, project: &str, stage: &str) -> u64 {
    limit_from(
        |k| app.kv(k),
        project,
        stage,
        app.cfg().factory.limits.stage_timeout_minutes,
    )
}
/// A stage that timed out gets a doubled limit next time, for that project and stage only.
pub(super) fn learn_timeout(app: &Shared, project: &str, stage: &str) {
    app.set_kv(
        &format!("stage_learned:{project}:{stage}"),
        Some(&(stage_minutes(app, project, stage) * 2).to_string()),
    );
}
