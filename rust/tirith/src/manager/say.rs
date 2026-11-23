//! What the owner, a room or an agent writes to a conversation.
use super::commands::owner_command;
use super::deliver::hand;
use super::evals::judge_evals;
use super::talk::talk;
use crate::app::Shared;
use crate::status;
/// A message from a room (a paired phone or another agent): shown in the chat of the room's project, handed to that project's manager, and the manager's answer to this turn goes back to that room.
pub fn from_room(app: &Shared, project: &str, room: &str, author: &str, text: &str) {
    let project = app.conv(project);
    let event = app.event(
        "",
        project,
        "room.message",
        "",
        &format!("{author} ({room}): {text}"),
        "",
    );
    let reacted = crate::turns::reacted(app, project, text);
    // What the owner writes from a paired phone is the owner's: owner-mood judges it (a correction counts), before it is in the chat; another agent's line never is.
    let _ = (!reacted && crate::rooms::paired_phone(author)).then(|| {
        super::owner_words::heard(app, project, text);
        judge_evals(app, project, "owner.message", text, event)
    });
    app.chat("room", &format!("{author} ({room}): {text}"), project);
    if reacted {
        return; // a 👍/👎 on the bot's last turn starts none
    }
    talk(project, |t| t.reply_to.insert(room.to_string()));
    let by = Some(("room".to_string(), String::new()));
    let text = with_cards(app, project, &format!("[room {room}] {author}: {text}"));
    hand(app, project, &text, true, by);
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
/// What `tirith say` and the page's chat write. `agent` is the role or lane of the OpenBot session it ran in (`TIRITH_ROLE`/`TIRITH_LANE`), empty for the owner: an agent's words go in the chat under its name and to the manager marked as its, never as an owner message (no `owner.message`, no owner-mood eval, no `/model`, which is the owner's alone).
pub fn say(app: &Shared, project: &str, text: &str, agent: &str) -> Result<(), String> {
    (project.is_empty() || app.conv(project) == project)
        .then_some(())
        .ok_or(format!("unknown project '{project}'"))?;
    let project = app.conv(project);
    let _ = (!agent.is_empty()).then(|| {
        (
            app.chat(agent, text, project),
            hand(
                app,
                project,
                &format!("[agent {agent}] {text}"),
                false,
                Some(("agent".into(), String::new())),
            ),
        )
    });
    let Some(()) = agent.is_empty().then_some(()) else {
        return Ok(());
    };
    if crate::turns::reacted(app, project, text) {
        app.chat("owner", text, project); // a 👍/👎 on the bot's last turn: no turn, no owner-mood
        return Ok(());
    }
    if status::asked(text) {
        app.chat("owner", text, project); // before the event, its evals and the trigger: each of them leads to a turn
        owner_command(app, project, text);
        return Ok(());
    }
    let event = app.event("", project, "owner.message", "", text, "");
    judge_evals(app, project, "owner.message", text, event); // before it is in the chat: else it looks said twice
    app.chat("owner", text, project);
    // A message that starts no turn (`/model`, `/status`, words kept while a task is taught) leaves no trigger behind.
    let _ = (!owner_command(app, project, text) && !crate::teach::heard(app, project, text)) // while it is taught a task, the owner's words go with the steps
        .then(|| {
            let by = Some(("owner".to_string(), String::new()));
            super::owner_words::heard(app, project, text);
            hand(app, project, &with_cards(app, project, text), true, by)
        });
    Ok(())
}
