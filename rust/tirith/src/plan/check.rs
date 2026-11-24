//! Item checks that can never pass.
use regex::{Captures, Regex};
use std::sync::LazyLock;

/// The tirith subcommands a check may call.
const OK: [&str; 19] = [
    "serve",
    "hook",
    "hooks",
    "status",
    "plan",
    "lane",
    "ask",
    "answer",
    "ask-owner",
    "card",
    "autopilot",
    "say",
    "room",
    "net",
    "models",
    "guard",
    "project",
    "service",
    "help",
];

/// Why an item's check can never pass: it calls a tirith subcommand that does not exist, or the removed `collector/`.
/// `tirith` is a command only at the start or after `;`, `&&`, `||`, `|` or `(` (also behind `!`, `do` or `then` there):
/// never as another command's argument, as in `cargo test -p tirith <name>` or in quoted text without a `$(`.
pub(super) fn check_problem(check: &str) -> Option<String> {
    static CALL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?:^|[\s;&|(])(cd\s+(?:\./)?collector(?:[/\s;&]|$))|(?:^|[;&|(])\s*(?:(?:!|do|then)\s+)?(?:tirith|neobot)\s+([A-Za-z-][\w-]*)",
        )
        .unwrap()
    });
    static QUOTED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"'[^']*'|"(?:[^"\\]|\\.)*""#).unwrap());
    let bare = QUOTED.replace_all(check, |q: &Captures| match &q[0] {
        text if text.starts_with('"') && text.contains("$(") => text.to_string(), // the shell runs it
        _ => "''".to_string(),
    });
    CALL.captures_iter(&bare).find_map(|c| {
        c.get(2).map_or_else(
            || Some("the check does `cd collector`, a removed folder (the daemon is Rust: use cargo test -q <name>)".to_string()),
            |s| (!OK.contains(&s.as_str())).then(|| format!("the check calls `tirith {}`, which is not a neobot command", s.as_str())),
        )
    })
}
