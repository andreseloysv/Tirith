//! The version the binary says, and when it is made again.
use std::path::Path;
use std::process::Command;

/// `TIRITH_VERSION`: the release's (`TIRITH_RELEASE`) or the crate's, then the commit when git knows it.
pub fn version(root: &Path) {
    println!("cargo::rerun-if-env-changed=TIRITH_RELEASE");
    // A new commit is a new version even when nothing carried changed: git writes this log at each one.
    if let Some(log) = git(root, &["rev-parse", "--git-path", "logs/HEAD"]) {
        let log = root.join(log);
        if log.is_file() {
            println!("cargo::rerun-if-changed={}", log.display());
        }
    }
    let release =
        std::env::var("TIRITH_RELEASE").unwrap_or_else(|_| crate::var("CARGO_PKG_VERSION"));
    let commit = git(root, &["rev-parse", "--short=8", "HEAD"])
        .map(|c| format!(" ({c})"))
        .unwrap_or_default();
    let target = crate::var("TARGET");
    println!("cargo::rustc-env=TIRITH_VERSION={release}{commit} {target}");
}

/// What git says in `root`, trimmed.
fn git(root: &Path, args: &[&str]) -> Option<String> {
    let mut git = Command::new("git");
    let out = git.arg("-C").arg(root).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}
