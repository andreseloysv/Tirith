//! Which shell calls of a shadow run act outward: a `tirith` command that is not a plain read (`say`, `ask`, `ask-owner`, `chat`, `safari open`, rooms, cards, lanes…), and commands that send, buy, delete or publish — a request that writes (curl with POST or data), mail, a push or a publish, ssh, `open`, AppleScript, or a deletion outside its worktree.
use regex::Regex;
use serde_json::Value;
use std::path::Path;
use std::sync::LazyLock;

/// A `tirith` command and its first word (a folder called tirith, like a server's home, is none).
static TIRITH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:(?:^|[;&|(`\n])\s*["']?(?:[^\s"']*/)?|[^\w./-]|bin/)tirith["']?\s+([a-z][a-z-]*)(?:\s+([a-z-]+))?"#).unwrap()
});
/// The `tirith` commands that only read or report on their own run: (command, its word, or any when empty).
const READS: [(&str, &str); 11] = [
    ("turn", "checks"),
    ("safari", "read"),
    ("status", ""),
    ("plan", "show"),
    ("lab", "list"),
    ("routine", "list"),
    ("room", "list"),
    ("room", "tail"),
    ("lane", "log"),
    ("net", "status"),
    ("net", "list"),
];
/// A command that sends or publishes, at the start of a command or after a separator.
static SENDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[;&|(`\n])\s*(?:sudo\s+)?(?:\S*/)?(?:git\s+push|(?:npm|pnpm|yarn|cargo|twine|gem|poetry)\s+publish|osascript|sendmail|mailx?|msmtp|ssh|scp|sftp|rsync|open|gh\s+(?:pr|issue|release|gist|repo|secret|workflow|run|label)\s+(?:create|comment|merge|close|delete|edit|review|upload|fork|archive|rename|reopen|set|rerun|cancel|enable|disable|run))(?:\s|$|;|\|)").unwrap()
});
/// A web request that writes: curl or wget with a method that changes something, data, a form or an upload; `gh api` with fields or a method.
static WRITES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:curl|wget)\b[^;&|]*\s(?:-[a-zA-Z]*X\s*'?(?:POST|PUT|PATCH|DELETE)|--request[\s=]'?(?:POST|PUT|PATCH|DELETE)|-[a-zA-Z]*[dFT]|--data|--form|--upload-file|--post-data|--post-file|--method=(?:POST|PUT|PATCH|DELETE))|\bgh\s+api\b[^;&|]*\s(?:-X|--method|-f|-F|--field|--raw-field|--input)\b").unwrap()
});
/// The commands that delete.
const DELETES: [&str; 6] = ["rm", "rmdir", "unlink", "trash", "shred", "srm"];

/// The command of a shell call.
fn command(tool: &str, input: &Value) -> Option<String> {
    let cmd = input["command"].as_str().or(input["cmd"].as_str());
    let shell = matches!(tool, "Bash" | "shell" | "exec_command" | "run_command");
    cmd.filter(|_| shell).map(String::from)
}

/// True for a `tirith` command in `cmd` that does more than read.
fn tirith_acts(cmd: &str) -> bool {
    TIRITH.captures_iter(cmd).any(|c| {
        let (first, word) = (&c[1], c.get(2).map_or("", |m| m.as_str()));
        !READS
            .iter()
            .any(|(a, b)| *a == first && (b.is_empty() || *b == word))
    })
}

/// True when `cmd` deletes something outside `dir`: an absolute path elsewhere, a home path, or one that climbs out.
fn deletes_outside(cmd: &str, dir: &Path) -> bool {
    cmd.split([';', '&', '|', '\n']).any(|part| {
        let words: Vec<&str> = part.split_whitespace().collect();
        let at = words.iter().position(|w| {
            let base = w.rsplit('/').next().unwrap_or(w);
            DELETES.contains(&base)
        });
        at.is_some_and(|i| {
            words[i + 1..].iter().any(|w| {
                let w = w.trim_matches(|c| c == '"' || c == '\'');
                let elsewhere = w.starts_with('/') && !Path::new(w).starts_with(dir);
                !w.starts_with('-') && (elsewhere || w.starts_with('~') || w.contains(".."))
            })
        })
    })
}

/// True for a shell call (under the rules' names) from the worktree `dir` that acts outward.
pub fn outward(tool: &str, input: &Value, dir: &Path) -> bool {
    command(tool, input).is_some_and(|c| {
        tirith_acts(&c) || SENDS.is_match(&c) || WRITES.is_match(&c) || deletes_outside(&c, dir)
    })
}
