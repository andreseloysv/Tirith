//! What the owner, a room or an agent writes to a conversation.
use super::deliver::{deliver, report};
use super::evals::judge_evals;
use super::talk::talk;
use crate::app::Shared;
use crate::models;
/// A message from a room (a paired phone or another agent): shown in the chat of the room's project, handed to that project's manager, and the manager's answer to this turn goes back to that room.
pub fn from_room(app: &Shared, project: &str, room: &str, author: &str, text: &str) {
    let project = app.conv(project);
    app.event(
        "",
        project,
        "room.message",
        "",
        &format!("{author} ({room}): {text}"),
        "",
    );
    app.chat("room", &format!("{author} ({room}): {text}"), project);
    talk(project, |t| t.reply_to.insert(room.to_string()));
    crate::turns::trigger(project, "room", "");
    deliver(
        app,
        project,
        &with_cards(app, project, &format!("[room {room}] {author}: {text}")),
        true,
    );
}
/// What the owner wrote, and the project's cards still open: an answer in plain words is then closed by the manager.
fn with_cards(app: &Shared, project: &str, text: &str) -> String {
    let sql = "SELECT id, question FROM owner_items WHERE status = 'open' AND project = ?1 ORDER BY rowid";
    let open: Vec<String> = app.query(sql, [project], |r| {
        Ok(format!(
            "{}: {}",
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?.chars().take(160).collect::<String>()
        ))
    });
    if open.is_empty() {
        return text.to_string();
    }
    format!("{text}\n\n[tirith] Cards still open here: {}. If this answers one, close it with tirith answer <card> \"<the answer>\".", open.join(" | "))
}
/// What `tirith say` and the page's chat write. `agent` is the role or lane of the Tirith session it ran in (`TIRITH_ROLE`/`TIRITH_LANE`), empty for the owner: an agent's words go in the chat under its name and to the manager marked as its, never as an owner message (no `owner.message`, no owner-mood eval, no `/model`, which is the owner's alone).
pub fn say(app: &Shared, project: &str, text: &str, agent: &str) -> Result<(), String> {
    (project.is_empty() || app.conv(project) == project)
        .then_some(())
        .ok_or(format!("unknown project '{project}'"))?;
    let project = app.conv(project);
    let _ = (!agent.is_empty()).then(|| {
        (
            app.chat(agent, text, project),
            deliver(app, project, &format!("[agent {agent}] {text}"), false),
        )
    });
    let Some(()) = agent.is_empty().then_some(()) else {
        return Ok(());
    };
    let event = app.event("", project, "owner.message", "", text, "");
    app.chat("owner", text, project);
    judge_evals(app, project, "owner.message", text, event);
    crate::turns::trigger(project, "owner", "");
    let _ = (!model_command(app, project, text) && !crate::teach::heard(app, project, text)) // while it is taught a task, the owner's words go with the steps
        .then(|| deliver(app, project, &with_cards(app, project, text), true));
    Ok(())
}
/// Only the local owner chat and authenticated paired phones call this path.
pub fn model_command(app: &Shared, project: &str, text: &str) -> bool {
    let Some(result) = models::command(app, text) else {
        return false;
    };
    report(
        app,
        project,
        &result.unwrap_or_else(|e| format!("Model change failed: {e}")),
    );
    true
}
