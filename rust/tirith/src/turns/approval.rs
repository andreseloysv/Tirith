//! Approvals: a turn that ends with its own approval card still open waits for the owner, and what the `unapproved` eval reads of a turn (its tools and the «Qué requiere aprobación» of the skills it read).
use super::record::Turn;
use super::versions::folder;
use crate::app::Shared;
use crate::server::s;

/// The skill template's section of what needs the owner's yes.
const SECTION: &str = "## Qué requiere aprobación";

/// An approval card opened during the turn in its conversation is still open at its end.
pub(super) fn waits(app: &Shared, t: &Turn) -> bool {
    app.query(
        "SELECT COUNT(*) FROM owner_items WHERE kind = 'approval' AND status = 'open' AND project = ?1 AND at >= ?2",
        [app.conv(&t.conv), t.started_at.as_str()],
        |r| r.get::<_, i64>(0),
    )
    .first()
    .is_some_and(|n| *n > 0)
}

/// The text of a skill's «Qué requiere aprobación» section, up to the next `## ` title.
pub(super) fn section(skill: &str) -> String {
    let Some(at) = skill.find(SECTION) else {
        return String::new();
    };
    let rest = &skill[at + SECTION.len()..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    rest[..end].split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A skill's SKILL.md: the bot's own, then the shared one.
fn skill_text(app: &Shared, t: &Turn, name: &str) -> String {
    [
        folder(app, t).join(".agents/skills"),
        app.home.join("bots/skills"),
    ]
    .iter()
    .map(|d| d.join(name).join("SKILL.md"))
    .find_map(|p| std::fs::read_to_string(p).ok())
    .unwrap_or_default()
}

/// What the event of a bot's turn adds for the `unapproved` eval: each tool call with its summary, and what each skill it read lists under «Qué requiere aprobación».
pub(super) fn context(app: &Shared, t: &Turn) -> String {
    let calls: Vec<String> = t
        .tools
        .iter()
        .map(|c| format!("{}: {}", s(c, "name"), s(c, "summary")))
        .map(|c| c.chars().take(200).collect())
        .collect();
    let mut text = String::new();
    let _ = (!calls.is_empty()).then(|| text += &format!(" · tool calls: {}", calls.join("; ")));
    for name in &t.skills {
        let needs = section(&skill_text(app, t, name));
        let _ = (!needs.is_empty())
            .then(|| text += &format!(" · «{name}» requires approval for: {needs}"));
    }
    text
}
