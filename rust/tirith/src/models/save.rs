//! Saving the owner's setting of a role, and its one-line form.
use super::roles::{role, role_with_source, RoleSetting};
use super::{state, updated::updated};
use crate::app::Shared;
use serde_json::Value;
/// Saves the owner's setting of a role (fields absent from `b` keep their effective value, "" clears them), or with `None` goes back to config.toml. A chat role whose setting changed restarts its sessions (after their current turn).
pub(super) fn save(
    app: &Shared,
    role_name: &str,
    b: Option<&Value>,
) -> Result<RoleSetting, String> {
    known_role(app, role_name)?;
    let before = role(app, role_name);
    let saved = b
        .map(|b| updated(app, role_name, b))
        .transpose()?
        .map(|s| serde_json::to_string(&s).unwrap_or_default());
    app.set_kv(&format!("models:{role_name}"), saved.as_deref());
    let (now, source) = role_with_source(app, role_name);
    app.event(
        "",
        "",
        "models",
        role_name,
        &format!("{role_name}: {} ({source})", line(&now)),
        "",
    );
    app.emit("models", state(app));
    let _ = (now != before && restarts(app, role_name)).then(crate::manager::restart);
    Ok(now)
}
/// The bots are factory roles too (scope = "own").
fn known_role(app: &Shared, role_name: &str) -> Result<(), String> {
    let roles = app.cfg().factory.roles();
    roles
        .iter()
        .any(|r| r == role_name)
        .then_some(())
        .ok_or_else(|| format!("role must be one of {}", roles.join(", ")))
}
/// True when a change of the role restarts the manager's sessions: a chat role or one of the owner's bots.
fn restarts(app: &Shared, role_name: &str) -> bool {
    let f = &app.cfg().factory;
    f.role::<String>(role_name, "session").as_deref() == Some("chat")
        || f.role::<String>(role_name, "scope").as_deref() == Some("own")
}
pub(super) fn line(s: &RoleSetting) -> String {
    format!(
        "agent={} model={} effort={}{}",
        s.agent,
        s.model.as_deref().unwrap_or("-"),
        s.effort.as_deref().unwrap_or("-"),
        if s.fallback.is_empty() {
            String::new()
        } else {
            format!(" fallback={}", s.fallback.join(","))
        }
    )
}
