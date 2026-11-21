//! Lines the measures' facts share: the owner's open questions, a paused project's reason and fix, an empty plan's fix.
use crate::app::Shared;
use crate::manager;
pub(super) fn owner_questions(app: &Shared) -> Vec<String> {
    manager::owner_open(app)
        .iter()
        .map(|o| {
            format!(
                "{}: {}",
                o["project"].as_str().unwrap_or(""),
                o["question"].as_str().unwrap_or("")
            )
            .trim_start_matches(": ")
            .to_string()
        })
        .collect()
}
/// Why the autopilot starts no lane for the project (the scheduler's last «paused» reason) and the command that fixes it.
pub(super) fn paused_text(project: &str, why: &str) -> String {
    format!(
        " [{project} paused: {why}; fix: {}]",
        crate::lanes::paused_fix(project, why)
    )
}
/// The project's paused note, or nothing when the scheduler did not pause it.
pub(super) fn paused_of(app: &Shared, project: &str) -> String {
    app.kv(&format!("paused:{project}"))
        .map(|w| paused_text(project, &w))
        .unwrap_or_default()
}
/// Autopilot projects with nothing the autopilot can start in the plan, with the command that adds work.
pub(super) fn empty_plan_note(empty: &[String]) -> String {
    empty
        .iter()
        .map(|p| {
            format!(
                " [{p} plan empty: nothing startable; fix: tirith plan add {p} <step> \"<text>\"]"
            )
        })
        .collect()
}
