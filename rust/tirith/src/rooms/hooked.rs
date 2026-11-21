//! What a bot's sealed webhook posts in its room: `tirith-hook <JSON>`, sealed by the platform's agent of that room.
use super::{handle::Seen, handle::Who, poll::Out};
/// True for a `tirith-hook` line: it is never a line for people (nor the owner's, nor another agent's). Only one from the agent that posts the room's webhook (`Seen::hook_from`) is handed on, as the bot's `webhook` event.
pub(super) fn hooked(seen: &Seen, who: &Who, text: &str, out: &mut Out) -> bool {
    let Some(body) = text.strip_prefix("tirith-hook ") else {
        return false;
    };
    let ours = !seen.hook_from.is_empty() && who.author == seen.hook_from && !seen.backlog;
    let _ = ours.then(|| out.hooks.push(body.to_string()));
    true
}
