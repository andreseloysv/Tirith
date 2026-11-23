//! The list of the owner's rooms the phones get (`tirith-rooms [{"r": room, "t": name, "o": role, "f": face}]`), one chat per project; `o` is the factory role that speaks in that room, so a phone edits the model of that role.
use super::Room;
use crate::{app::Shared, config::get as cfg_get};
use serde_json::{json, Value};
/// One room of the list: the face of the role that carries the conversation, a project's room keeping the Project Manager's shape in the project's own colour (a phone page that does not know `f` ignores it).
pub(super) fn listed_room(
    raw: &toml::Table,
    id: &str,
    title: &str,
    role: &str,
    project: Option<&crate::config::Project>,
) -> Value {
    let get =
        |k: &str| cfg_get::<String>(raw, &["roles", role, "face", k]).filter(|x| !x.is_empty());
    let color = project
        .map(crate::projects::color_for)
        .or_else(|| get("color"))
        .unwrap_or_else(|| "#3b82f6".into());
    json!({"r": id, "t": title, "o": role, "f": {"shape": get("shape").unwrap_or_else(|| "round".into()), "color": color}})
}
/// The name a bot's room has on the phones, or "All projects" for what belongs to none.
fn bot(app: &Shared, project: &str) -> String {
    crate::bots::bot(app, project)
        .map_or("All projects".to_string(), |b| crate::server::s(&b, "name"))
}
/// The name a room has on the phones: its project's, else its bot's.
pub(super) fn label(app: &Shared, project: &str) -> String {
    app.cfg().project(project).map_or_else(
        || bot(app, project),
        |p| {
            if p.name.is_empty() {
                p.id.clone()
            } else {
                p.name.clone()
            }
        },
    )
}
/// The list of the manager's own rooms, as the phones get it.
pub(super) fn list<'a>(app: &Shared, rooms: impl Iterator<Item = &'a Room>) -> String {
    let raw = app.cfg().factory.raw.clone();
    json!(rooms
        .filter(|r| r.kind == "manager")
        .map(|r| listed_room(
            &raw,
            &r.id,
            &label(app, &r.project),
            &app.chat_role(&r.project),
            app.cfg()
                .project(&r.project)
                .filter(|_| crate::bots::bot(app, &r.project).is_none())
        ))
        .collect::<Vec<_>>())
    .to_string()
}
