//! Item checks that can never pass.
use regex::Regex;
use std::sync::LazyLock;

/// The tirith subcommands a check may call.
const OK: [&str; 18] = [
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
    "help",
];

/// Why an item's check can never pass: it calls a tirith subcommand that does not exist, or the removed `collector/`.
pub(super) fn check_problem(check: &str) -> Option<String> {
    static CALL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?:^|[\s;&|(])(?:(cd\s+(?:\./)?collector(?:[/\s;&]|$))|tirith\s+([A-Za-z-][\w-]*))",
        )
        .unwrap()
    });
    CALL.captures_iter(check).find_map(|c| {
        c.get(2).map_or_else(
            || Some("the check does `cd collector`, a removed folder (the daemon is Rust: use cargo test -q <name>)".to_string()),
            |s| (!OK.contains(&s.as_str())).then(|| format!("the check calls `tirith {}`, which is not a tirith command", s.as_str())),
        )
    })
}
