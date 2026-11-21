//! The text of config.toml with the bots laid into it.
use super::{fragment, layer, sweep};
use crate::server::s;
use serde_json::Value;
use toml_edit::DocumentMut;
/// A bot's conversation key: its `key`, or `bot-<id>` for a new one.
pub(super) fn key_of(b: &Value) -> String {
    Some(s(b, "key"))
        .filter(|k| !k.is_empty())
        .unwrap_or_else(|| format!("bot-{}", s(b, "id")))
}
/// `bots` has the bot `to` (with `changed`, one whose routes are (re)written here: every owner's bot, and a system bot the owner changed).
pub(super) fn has(bots: &[Value], to: &str, changed: bool) -> bool {
    bots.iter()
        .any(|b| (!changed || b["system"] != true || b["dirty"] == true) && key_of(b) == to)
}
/// The file's own sections, then the bots' fragment after them, no empty header behind.
fn joined(head: String, frag: String) -> String {
    if frag.is_empty() {
        head
    } else if head.trim().is_empty() {
        frag
    } else {
        format!("{}\n\n{frag}", head.trim_end())
    }
}
/// The text of config.toml (`text` as it is) with the bots laid into it by toml_edit, so its other sections and their comments stay as they are: each of the owner's bots a `[roles.bot-<id>]` that extends the manager with its own scope, name, face, session and access, each routine a `[[routes]]` on `schedule` and each of its routes a `[[routes]]` of its own; a bot not in `bots` goes, with its routes. A system bot has no role of its own here: only what the owner changed of it (`dirty`: its name, face, show, session and access, and its routes with them) is laid into its factory role's table (a real table, created with `entry` — an inline one would make a later `[roles.bot-…]` header unparseable) as a layer over the factory, its other keys (a model, say) untouched; an untouched system bot's routes in the file stay as they are.
pub(super) fn written(text: &str, bots: &[Value]) -> Result<String, String> {
    let mut doc: DocumentMut = text.parse().map_err(|e| format!("config.toml: {e}"))?;
    // Every bot role and schedule route goes out of the file (and a changed bot's other routes, written again with it); the bots are written again after the owner's sections, together at the end.
    sweep::drop_bots(&mut doc, bots);
    bots.iter()
        .filter(|b| b["system"] == true && b["dirty"] == true)
        .for_each(|b| layer::lay_system(&mut doc, b));
    sweep::drop_empty(&mut doc);
    let frag = fragment::fragment(bots)?;
    Ok(joined(doc.to_string(), frag))
}
