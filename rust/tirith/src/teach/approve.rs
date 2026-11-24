//! «Aprobar» on the tab «Enseñar»: the owner's, for a draft the bot rehearsed (a finished rehearsal with a step that worked): it moves from the drafts folder to the skills, without `draft: true`, so from then on the bot uses it.
use super::caller::own;
use super::rehearsal;
use crate::app::Shared;
use crate::bots::{activated, is_draft};
use crate::server::s;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};
/// A skill's name is lowercase letters, digits and dashes.
fn named(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}
/// The text of the bot's own skill `name`, a draft or not (None for a name that is not one).
pub(super) fn text_of(app: &Shared, key: &str, name: &str) -> Option<String> {
    named(name).then_some(())?;
    crate::bots::sweep_drafts(app, key);
    [
        crate::bots::draft_file(app, key, name),
        crate::bots::skill_file(app, key, name),
    ]
    .iter()
    .find_map(|f| std::fs::read_to_string(f).ok())
}
/// Whether a rehearsal of `skill` is over and had at least one step that worked: what the owner saw before approving.
fn rehearsed(app: &Shared, key: &str, skill: &str) -> bool {
    rehearsal::all(app, key).iter().any(|r| {
        s(r, "skill") == skill
            && !rehearsal::live(r)
            && r["steps"]
                .as_array()
                .is_some_and(|v| v.iter().any(|st| st["ok"] == true))
    })
}
/// Activates the bot's own draft `skill`: moved from its drafts folder to its skills, without the `draft: true` line. The bot is told with its next message: its live session saw a draft.
pub(super) fn approve_skill(app: &Shared, key: &str, skill: &str) -> Result<(), String> {
    named(skill)
        .then_some(())
        .ok_or("a skill's name is lowercase letters, digits and dashes")?;
    let text = text_of(app, key, skill).ok_or(format!("no skill «{skill}»"))?;
    let held = crate::bots::draft_file(app, key, skill);
    (held.exists() || is_draft(&text))
        .then_some(())
        .ok_or(format!("«{skill}» is not a draft"))?;
    rehearsed(app, key, skill).then_some(()).ok_or(format!(
        "«{skill}» has not been rehearsed: the bot rehearses it in your Edge first (neobot teach rehearse), and at least one of its steps must have worked"
    ))?;
    let file = crate::bots::skill_file(app, key, skill);
    let done = std::fs::create_dir_all(file.parent().unwrap_or(&file))
        .and_then(|_| std::fs::write(&file, activated(&text)))
        .and_then(|_| std::fs::remove_dir_all(held.parent().unwrap_or(&held)).or(Ok(())));
    done.map_err(|e| format!("{}: {e}", file.display()))?;
    crate::bots::versioned(app, key, "library"); // a change of its skills from the Library's side
    crate::manager::noted(
        app,
        key,
        &format!("The owner approved «{skill}»: it is no longer a draft, use it from now on."),
    );
    app.emit("library", json!({"bot": key}));
    Ok(())
}
/// `POST /api/teach/<id>/approve` with `{skill}`: from the page.
pub(super) async fn approve(
    State(app): State<Shared>,
    who: crate::server::Who,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(b): Json<Value>,
) -> Response {
    let key = match own(&app, &who, &h, &id) {
        Ok(_) if who.is_agent() => {
            let e =
                "approving is the owner's: it comes from the tab «Enseñar», never from an agent";
            return (StatusCode::FORBIDDEN, Json(json!({"error": e}))).into_response();
        }
        Ok(k) => k,
        Err(r) => return *r,
    };
    match approve_skill(&app, &key, &s(&b, "skill")) {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e}))).into_response(),
    }
}
