//! What `neobot browser` adds when NeoBot's own browser is the one chosen and cannot run here (its helper is
//! not in this home, or there is no Node, as on a new install): an agent told to act in it would look for
//! browser tools it was never given, and say something untrue.
use super::choice::{chosen, OWN};
use crate::app::Shared;

/// Nothing while the chosen browser can be used.
pub fn note(app: &Shared) -> String {
    if chosen(app) != "neobot" || crate::models::own_browser(app) {
        return String::new();
    }
    let own = OWN.join(" or ");
    format!(" — but NeoBot's own browser is not set up on this computer, so you have no browser tools: say so. The owner's own browser here is {own}: a page they have open there you read with neobot <name> read, and when they ask you to work there run neobot browser use <name>")
}
