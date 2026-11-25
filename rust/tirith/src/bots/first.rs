//! A new home starts with one bot, NeoBot, with the app icon's own look, and nothing else to look at: made
//! once, when the home has no project and no bot of the owner's (an older home keeps what it has). It owes
//! the owner its welcome (web/js/views/onboarding/) until `intro.rs` writes it down or the owner skips it.
use super::card::save;
use super::list::{all, key_for};
use crate::app::Shared;
use crate::server::s;
use serde_json::json;

/// kv: the first bot's id once made, "none" for a home that was not new.
const MADE: &str = "first_bot";
/// kv: the id of the bot that still owes its welcome.
pub(super) const WELCOME: &str = "welcome";

/// At start: the first bot, once. A home where it could not be written gets it at the next start.
pub(super) fn start(app: &Shared) {
    if app.kv(MADE).is_some() {
        return;
    }
    let new = app.cfg().projects.is_empty() && all(app).iter().all(|b| b["system"] == true);
    if !new {
        return app.set_kv(MADE, Some("none"));
    }
    let look = json!({"name": "NeoBot", "shape": "blob", "color": "#8fd14f"});
    if let Ok(bot) = save(app, look) {
        let id = s(&bot, "id");
        app.set_kv(WELCOME, Some(&id));
        app.set_kv(MADE, Some(&id));
    }
}

/// The bot that still owes its welcome, while it exists.
pub(super) fn welcome(app: &Shared) -> Option<String> {
    app.kv(WELCOME).filter(|id| key_for(app, id).is_some())
}

/// The welcome of bot `id` was said, or skipped.
pub(super) fn welcomed(app: &Shared, id: &str) {
    if app.kv(WELCOME).as_deref() == Some(id) {
        app.set_kv(WELCOME, None);
    }
}

#[cfg(test)]
#[path = "../../checks/first_bot.rs"]
mod tests;
