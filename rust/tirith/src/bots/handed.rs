//! What a bot is handed with its next message: the memory and skills of the folder that turn runs in. A skill that changed since its last turn (the Library, an experiment adopted) is read again in a session of its own: a resumed one goes on doing what the old one said.
use super::context::context_at;
use super::folder::{folder, skills, SKILLS};
use crate::app::Shared;
use std::collections::BTreeMap;
use std::path::Path;

/// The message's context. `run`: a routine's run, in its experiment's folder when it has one (anything else runs in the bot's own, whatever run is on its way). `says`: a message really goes.
pub fn handed(app: &Shared, key: &str, run: bool, says: bool) -> Option<String> {
    let main = app.home.join("manager").join(key);
    let dir = if run { folder(app, key) } else { main.clone() };
    let text = context_at(app, key, &dir)?;
    let changed = match says && dir == main {
        true => changed(app, key, &main),
        false => vec![], // an experiment's B is the branch's, for that turn alone
    };
    if changed.is_empty() {
        return Some(text);
    }
    crate::lab::fresh_session(app, key);
    Some(format!(
        "{text}\n[tirith] Changed since your last turn: {}. Read each one's SKILL.md again before you use it: what you did before followed the old one.",
        changed.join(", ")
    ))
}

/// Each skill the bot may use (its own before a shared one of the same name), by the hash of its text.
fn now(app: &Shared, dir: &Path) -> BTreeMap<String, String> {
    let shared = skills(&app.home.join("bots/skills"), true);
    let own = skills(&dir.join(SKILLS), false);
    let hash = |text: &str| crate::rooms::sealed::sha_hex(text.as_bytes());
    shared
        .into_iter()
        .chain(own)
        .map(|(name, text)| (name, hash(&text)))
        .collect()
}

/// The skills changed or gone since the last message; a new one is no change (no session learned it otherwise).
pub(super) fn changed(app: &Shared, key: &str, dir: &Path) -> Vec<String> {
    let kept = format!("skills:{key}");
    let (was, is) = (app.kv(&kept), now(app, dir));
    let text = serde_json::to_string(&is).unwrap_or_default();
    if was.as_deref() != Some(text.as_str()) {
        app.set_kv(&kept, Some(&text));
    }
    let was: BTreeMap<String, String> = was
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default();
    was.into_iter()
        .filter(|(name, hash)| is.get(name) != Some(hash))
        .map(|(name, _)| name)
        .collect()
}
