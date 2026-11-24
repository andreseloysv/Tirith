//! The other `codex` subcommands that run commands: their sandbox does not nest in NeoBot's on macOS, and the caller is told.
use super::flags::{option, valued};

/// What is said of `args`, and whether Codex is not started at all.
pub(super) fn said(args: &[String]) -> Option<(String, bool)> {
    match word(args)? {
        "sandbox" => Some((
            "`codex sandbox` has none to give inside NeoBot's on macOS (sandboxes do not nest): run the command itself, or use codex exec".into(),
            true,
        )),
        word @ ("app-server" | "exec-server" | "mcp-server" | "resume" | "fork" | "cloud" | "queue") => Some((
            format!("macOS keeps one sandbox, so `codex {word}` runs no command inside NeoBot's: use codex exec or codex review for that"),
            false,
        )),
        _ => None,
    }
}

/// The subcommand: the first word that is no option or an option's value.
fn word(args: &[String]) -> Option<&str> {
    let mut i = 0;
    while let Some(word) = args.get(i) {
        i += 1;
        match option(word) {
            Some((name, None)) if valued(name) => i += 1,
            Some(_) => {}
            None if word.starts_with('-') => {}
            None => return Some(word),
        }
    }
    None
}
