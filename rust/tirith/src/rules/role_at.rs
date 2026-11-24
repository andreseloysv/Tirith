//! The chat role a hook belongs to, read from its folder.
use crate::app::Shared;

/// The role whose folder under NeoBot's home holds `cwd` — `<home>/<role>/…`, a bot's `<home>/manager/<role>`, and a project's chat in `<home>/manager/<project>` (the manager) — never a lane under `<home>/lanes` (no role of that name runs a chat or a turn there). It names a hook's chat role when the agent passed none on to its hooks.
pub fn role_at(app: &Shared, cwd: &str) -> Option<String> {
    let rel = std::path::Path::new(cwd).strip_prefix(&app.home).ok()?;
    let mut parts = rel.iter().map(|c| c.to_string_lossy().into_owned());
    let known = |r: &str| {
        app.cfg().factory.roles().iter().any(|x| x == r)
            && app.cfg().factory.role::<String>(r, "session").as_deref() != Some("lane")
    };
    match parts.next().as_deref() {
        Some("manager") => Some(
            parts
                .next()
                .filter(|r| known(r))
                .unwrap_or_else(|| "manager".into()),
        ),
        Some(r) if known(r) => Some(r.to_string()),
        _ => None,
    }
}
