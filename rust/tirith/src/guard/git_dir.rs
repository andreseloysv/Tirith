//! Where a repository keeps its hooks.
use std::path::PathBuf;

/// The repo's common git dir, by reading `.git` (a folder, or a file pointing into a worktree's git dir).
pub(super) fn git_dir(repo: &std::path::Path) -> Option<PathBuf> {
    let dot = repo.join(".git");
    let Some(()) = (!dot.is_dir()).then_some(()) else {
        return Some(dot);
    };
    let text = std::fs::read_to_string(&dot).ok()?;
    let dir = repo.join(text.strip_prefix("gitdir:")?.trim());
    Some(
        std::fs::read_to_string(dir.join("commondir"))
            .map_or_else(|_| dir.clone(), |c| dir.join(c.trim())),
    )
}
