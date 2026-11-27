//! A call through the gateway from a run that is not the real one: a shadow B run (conversation `shadow-<experiment>-<pair>`) or a replay's (`replay-…`, or a token `replay:…`). Its reads pass; a write is held back, never run, and recorded as held back, so the comparison still sees what it would have done. A tool is a read when its server declares it read-only (`readOnlyHint`) or a word of its name says it reads, and no word of its name sends, writes or deletes; anything else is a write.
use serde_json::Value;

/// The kind of run `who` (`<role>@<conversation>`, as its token says) is, when it is not the real one.
fn kind(who: &str) -> Option<&'static str> {
    let conv = who.rsplit_once('@').map_or("", |(_, c)| c);
    if who.starts_with("replay:") || conv.starts_with("replay-") {
        Some("replay")
    } else {
        conv.starts_with("shadow-").then_some("shadow run")
    }
}

/// Why a call of `connector`'s `tool` (its catalog entry `entry`) from the run `who` is held back, with the word its record uses; None when it runs.
pub fn held_back(
    who: &str,
    connector: &str,
    tool: &str,
    entry: Option<&Value>,
) -> Option<(String, String)> {
    let kind = kind(who)?;
    let read = entry.is_some_and(|t| t["annotations"]["readOnlyHint"] == true);
    crate::lab::name_acts(tool, read).then(|| {
        let word = format!("held back in a {kind}");
        let said = format!("{word}: {connector}__{tool}. It changes something outside NeoBot, so it did not run; say what you would have done and go on.");
        (said, word)
    })
}
