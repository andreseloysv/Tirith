//! A bot's webhook as NeoBot keeps it (kv `webhook:<bot>`: the platform's id, url and from, and the room), and what arrives through it.
use super::hook_clean::{ident, quoted};
use super::hook_event::event_of;
use super::hook_fire::{fire, matching};
use super::list::bot;
use crate::app::{App, Shared};
use crate::server::s;
use serde_json::Value;
/// The kept webhook of the bot whose conversation is `key`, if it has one.
pub(super) fn kept(app: &App, id: &str) -> Option<Value> {
    serde_json::from_str(&app.kv(&format!("webhook:{id}"))?).ok()
}
/// The key of the platform's agent that posts the webhook of conversation `key`'s room (empty when its bot has no webhook): only its `tirith-hook` lines are the bot's events.
pub fn hook_from(app: &App, key: &str) -> String {
    bot(app, key)
        .and_then(|b| kept(app, &s(&b, "id")))
        .map(|h| s(&h, "from"))
        .unwrap_or_default()
}
/// A `tirith-hook` line of bot `key`'s webhook (the JSON after the mark): its `webhook` event is kept and shown, and runs each of its webhook routines whose `when` holds. Never a line of the owner or of another agent.
pub fn webhook(app: &Shared, key: &str, payload: &str) {
    let (Some(b), Some(e)) = (bot(app, key), event_of(payload)) else {
        return;
    };
    let hits = matching(&b, &e);
    let shown = format!(
        "Webhook · {} · {}",
        ident(&super::hook_event::label(&e)),
        quoted(&s(&e, "title"))
    );
    let _ = hits
        .is_empty()
        .then(|| app.event("", key, "webhook", "", &shown, ""));
    hits.iter()
        .for_each(|r| fire(app, &b, r, &e, "event:webhook"));
}
