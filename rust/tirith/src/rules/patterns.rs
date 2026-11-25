//! The shell command shapes the rules look for.
use regex::Regex;
use std::sync::LazyLock;

pub(super) static GUARD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"--no-verify|\bgit\b[^;&|]*\bcommit\b[^;&|]*\s-[a-zA-Z]*n\b|core\.hooksPath")
        .unwrap()
});
/// A shell command that changes files: with `.git/hooks` in it, it is refused (reading the hooks stays allowed).
pub(super) static WRITES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(rm|mv|cp|ln|chmod|chflags|chown|tee|truncate|touch|install|unlink|rsync)\b|>|\b(sed|perl)\s+-[a-zA-Z]*i").unwrap()
});
pub(super) static MAIN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bgit\b[^;&|]*\b(push|update-ref)\b|\bgit\b[^;&|]*\b(checkout|switch)\s+(main|master)\b|\bgit\b[^;&|]*\bbranch\s+-[fDM]\b[^;&|]*\b(main|master)\b").unwrap()
});
pub(super) static WAIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bsleep\s+(?:[5-9]|[1-9]\d+)(?:\.\d+)?s?\b|\bsleep\s+[1-9]\d*(?:\.\d+)?[mhd]\b")
        .unwrap()
}); // sleep >= 5s; a loop's sleep is `loop_sleeps`
/// The words that open and close a shell loop, and `sleep`.
static LOOPS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:for|while|until|done|sleep)\b").unwrap());
/// A sleep inside a loop, between its `for`, `while` or `until` and its `done`: waiting by asking again and again. A short sleep after the loop has ended is no part of it.
pub(super) fn loop_sleeps(cmd: &str) -> bool {
    let mut open = 0u32;
    for word in LOOPS.find_iter(cmd) {
        match word.as_str() {
            "done" => open = open.saturating_sub(1),
            "sleep" if open > 0 => return true,
            "sleep" => {}
            _ => open += 1,
        }
    }
    false
}
/// Following or watching something until it happens, only in command position (at the start or after `;`, `&&`, `||`, `|`, `(`, `$(`, `do` or `then`), with quoted text taken out first (`QUOTED`) so a message that says «watch» or «wait» passes: `tail` with `-f`/`-F`/`--follow` (in a joined cluster too, as `-fn0` or `-n0F`), `watch`, `fswatch`, `inotifywait`, the shell's `wait`, `gh run watch`, and any `--watch`.
pub(super) static QUOTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)'(?:[^'\\]|\\.)*'|"(?:[^"\\]|\\.)*""#).unwrap());
pub(super) static FOLLOW: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[;&|(\n]|\b(?:do|then)\b)\s*(?:\S*/)?(?:(?:watch|fswatch|inotifywait|wait)(?:$|[\s;&|()])|tail\b[^;&|()\n]*?(?:\s--follow(?:[=\s]|$)|\s-[A-Za-z0-9]*[fF][A-Za-z0-9]*)|gh\s+run\s+watch(?:$|[\s;&|()]))|(?:^|\s)--watch(?:[=\s]|$)").unwrap()
});
/// `neobot hooks remove` in command position or after a path: taking NeoBot's hooks out of the agents.
pub(super) static UNHOOK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:neobot|tirith)(?:\.exe)?\s+hooks\s+remove\b").unwrap());
