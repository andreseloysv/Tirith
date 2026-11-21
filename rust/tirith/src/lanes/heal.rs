//! What earlier cleanups moved into a project but git still sees as new gets its folder excluded.
use super::exclude::ensure_excluded;
use crate::app::git;
use std::path::Path;

/// The paths the project's `info/exclude` names, as `/<path>` lines.
fn moved_lines(repo: &Path, gitdir: &str) -> Vec<String> {
    std::fs::read_to_string(repo.join(gitdir).join("info/exclude"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            l.strip_prefix('/')
                .map(|r| r.trim_end_matches('/').to_string())
        })
        .collect()
}
/// A `\`, space or `[` in a moved file's name broke its per-file exclude line: git still lists it as new, so its folder is excluded instead.
pub(super) async fn heal_excludes(repo: &Path, gitdir: &str) {
    let moved = moved_lines(repo, gitdir);
    let fresh = git(repo, &["ls-files", "--others", "--exclude-standard", "-z"])
        .await
        .unwrap_or_default();
    for rel in fresh.split('\0').filter(|r| {
        !r.is_empty()
            && moved
                .iter()
                .any(|m| m == r || r.starts_with(&format!("{m}/")))
    }) {
        ensure_excluded(
            repo,
            Path::new(rel)
                .parent()
                .filter(|q| !q.as_os_str().is_empty())
                .unwrap_or(Path::new(rel)),
        )
        .await;
    }
}
