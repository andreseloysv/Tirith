//! «Aprobar» on the tab «Enseñar»: the draft rehearsed loses `draft: true` from its header, so from then on the bot uses it.
use super::rehearse_api::own;
use crate::app::Shared;
use crate::server::s;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};
/// The skill's text without the `draft: true` line of its header; None when it has none.
pub(super) fn activated(text: &str) -> Option<String> {
    let body = text.strip_prefix("---\n")?;
    let end = body.find("\n---")?;
    let head: Vec<&str> = body[..end].lines().collect();
    let kept: Vec<&str> = head
        .iter()
        .copied()
        .filter(|l| l.trim() != "draft: true")
        .collect();
    (kept.len() < head.len()).then(|| format!("---\n{}{}", kept.join("\n"), &body[end..]))
}
/// A skill's name is lowercase letters, digits and dashes.
fn named(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}
/// Activates the bot's own draft `skill`.
pub(super) fn approve_skill(app: &Shared, key: &str, skill: &str) -> Result<(), String> {
    named(skill)
        .then_some(())
        .ok_or("a skill's name is lowercase letters, digits and dashes")?;
    let file = crate::bots::skill_file(app, key, skill);
    let text = std::fs::read_to_string(&file).map_err(|_| format!("no skill «{skill}»"))?;
    let out = activated(&text).ok_or(format!("«{skill}» is not a draft"))?;
    std::fs::write(&file, out).map_err(|e| format!("{}: {e}", file.display()))?;
    crate::bots::versioned(app, key, "library"); // a change of its skills from the Library's side
    app.chat(
        "tirith",
        &format!("The owner approved «{skill}»: it is no longer a draft, use it from now on."),
        key,
    );
    app.emit("library", json!({"bot": key}));
    Ok(())
}
/// `POST /api/teach/<id>/approve` with `{skill}`: from the page.
pub(super) async fn approve(
    State(app): State<Shared>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(b): Json<Value>,
) -> Response {
    let key = match own(&app, &h, &id) {
        Ok(k) => k,
        Err(r) => return *r,
    };
    match approve_skill(&app, &key, &s(&b, "skill")) {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e}))).into_response(),
    }
}
