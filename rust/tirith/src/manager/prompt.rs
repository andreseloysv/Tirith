//! What a conversation's agent is told besides its messages: its role's instructions and the recent chat.
use crate::app::Shared;
/// The factory prompt of this conversation's role (a bot's is its role's, with the shared skills folder filled in).
pub(super) fn prompt(app: &Shared, project: &str) -> String {
    let text = instructions(app, &app.chat_role(project));
    app.cfg().project(project).map_or(text.clone(), |p| {
        text.replace("{id}", &p.id)
            .replace("{repo}", &p.repo().display().to_string())
    })
}
/// A role's instructions, wherever its turn runs (its conversation, a loose turn of the lab).
pub(super) fn instructions(app: &Shared, role: &str) -> String {
    app.cfg().factory.prompts_of(role).replace(
        "{shared}",
        &app.home.join("bots/skills").display().to_string(),
    )
}
/// What a fresh session of `project` is handed before its first message: the recent chat, but in a run of an experiment, where each side would read the other's folder and calls there and do the same.
pub(super) fn opening(app: &Shared, project: &str) -> String {
    if crate::lab::experimenting(app, project) {
        return String::new();
    }
    history(app, project)
}
pub(super) fn history(app: &Shared, project: &str) -> String {
    let rows: Vec<String> = app.query(
        "SELECT role, text FROM chat WHERE project = ?1 ORDER BY id DESC LIMIT 30",
        [project],
        |r| {
            Ok(format!(
                "{}: {}",
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?
            ))
        },
    );
    format!(
        "Recent conversation for context (do not repeat completed work):\n{}\n\nNew messages:\n",
        rows.into_iter().rev().collect::<Vec<_>>().join("\n")
    )
}
