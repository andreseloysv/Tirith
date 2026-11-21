//! At start, saved settings that cannot run are said in the general manager's chat.
use super::owners::mismatch;
use super::roles::role_with_source;
use crate::app::Shared;
/// At start: every role whose saved setting pairs an agent with another agent's model is said once in the general manager's chat, so it is chosen again before a lane or a turn fails on it.
pub fn start(app: Shared) {
    let bad: Vec<String> = app
        .cfg()
        .factory
        .roles()
        .into_iter()
        .filter_map(|r| crossed(&app, r))
        .collect();
    let _ = (!bad.is_empty()).then(|| {
        crate::manager::emit_event(
            &app,
            "",
            "factory.invalid",
            &format!(
                "Saved model settings that cannot run — {}. Choose them again in Settings.",
                bad.join("; ")
            ),
            None,
        )
    });
}
/// "<role>: <why>" when the role's own setting (not a bot's borrowed one) cannot run.
fn crossed(app: &Shared, r: String) -> Option<String> {
    let cfg = app.cfg();
    Some(role_with_source(app, &r))
        .filter(|(_, src)| *src != "managers")
        .and_then(|(s, _)| mismatch(&cfg, &s.agent, s.model.as_deref()))
        .map(|e| format!("{r}: {e}"))
}
