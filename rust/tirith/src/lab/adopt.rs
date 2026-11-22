//! Adopting a branch: merged into the bot's main without losing a note someone wrote in memory.md while the experiment ran.
use super::git::{git, merge};
use std::path::Path;
use std::sync::LazyLock;

/// The lines the main at `dir` added to memory.md since it was branched, as written.
fn added(dir: &Path, branch: &str) -> Vec<String> {
    let base = git(dir, &["merge-base", "HEAD", branch]).unwrap_or_default();
    let diff = git(dir, &["diff", &base, "HEAD", "--", "memory.md"]).unwrap_or_default();
    let lines = diff
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"));
    lines
        .map(|l| l[1..].to_string())
        .filter(|l| !l.trim().is_empty())
        .collect()
}

static TAG: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\[[ms][0-9]+\][ \t]*").unwrap());

/// A note without its number: a note that comes back is numbered again at the next turn, so two never share one.
fn bare(line: &str) -> String {
    TAG.replace(line, "").trim().to_string()
}

/// Merges `branch` into the main at `dir` (`merge`: where main moved on the same lines the branch's tested text wins), then writes back, as new notes, what main added to memory.md that the merge lost. False, and nothing merged, when it cannot merge.
pub(super) fn adopt(dir: &Path, branch: &str, msg: &str) -> bool {
    let mine = added(dir, branch);
    if !merge(dir, branch, msg) {
        return false;
    }
    let file = dir.join("memory.md");
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    let kept: Vec<String> = text.lines().map(bare).collect();
    let lost: Vec<String> = mine
        .iter()
        .map(|l| bare(l))
        .filter(|l| !kept.contains(l))
        .collect();
    if !lost.is_empty() {
        let body = text.trim_end().to_string() + "\n" + &lost.join("\n") + "\n";
        let _ = std::fs::write(&file, body);
        let _ = git(
            dir,
            &[
                "commit",
                "-q",
                "-am",
                "lab: keep the notes written while the experiment ran",
            ],
        );
    }
    true
}
