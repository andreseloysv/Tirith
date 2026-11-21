//! Git in a bot's repository, as Tirith: its own name, no signing.
use std::path::Path;
use std::process::{Command, Stdio};

/// `git -C dir args…`: its stdout trimmed, or what it said on stderr when it failed.
pub(super) fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=Tirith",
            "-c",
            "user.email=tirith@localhost",
        ])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    let said = |b: &[u8]| String::from_utf8_lossy(b).trim().to_string();
    if out.status.success() {
        Ok(said(&out.stdout))
    } else {
        Err(format!("git {}: {}", args.join(" "), said(&out.stderr)))
    }
}

/// The one commit's subject of `branch` in the repository at `dir`: it must exist, have exactly one commit over the checked-out main, and be checked out nowhere.
pub(super) fn one_commit(dir: &Path, branch: &str) -> Result<String, String> {
    let have = |r: &str| git(dir, &["rev-parse", "--verify", "-q", r]).is_ok();
    have(&format!("refs/heads/{branch}"))
        .then_some(())
        .ok_or(format!("no branch '{branch}' in {}", dir.display()))?;
    let count = git(dir, &["rev-list", "--count", &format!("HEAD..{branch}")])?;
    (count == "1").then_some(()).ok_or(format!(
        "'{branch}' has {count} commits over the bot's main: an experiment is exactly one"
    ))?;
    let out = git(dir, &["worktree", "list", "--porcelain"])?;
    let held = out
        .lines()
        .any(|l| l == format!("branch refs/heads/{branch}"));
    (!held).then_some(()).ok_or(format!(
        "'{branch}' is checked out in a worktree: remove that worktree first"
    ))?;
    git(dir, &["log", "-1", "--format=%s", branch])
}

/// The worktree goes and the branch with it: what an experiment leaves when it ends, adopted or not.
pub(super) fn clear(dir: &Path, tree: &Path, branch: &str) {
    let path = tree.display().to_string();
    let _ = git(dir, &["worktree", "remove", "--force", &path]);
    let _ = std::fs::remove_dir_all(tree);
    let _ = git(dir, &["worktree", "prune"]);
    let _ = git(dir, &["branch", "-D", branch]);
}
