//! A lane's git-ignored files: build caches stay behind, the rest moves to the project without overwriting anything.
use std::path::{Path, PathBuf};

/// Folders git ignores that are only build caches: they go with the worktree and are never moved to the project.
pub(super) fn is_cache(rel: &Path) -> bool {
    rel.components().any(|c| {
        [
            "target",
            "node_modules",
            ".build",
            "build",
            ".gradle",
            "__pycache__",
            "venv",
            ".venv",
        ]
        .contains(&c.as_os_str().to_str().unwrap_or(""))
    })
}
/// The same path with the lane's id in its file name (`doc-<lane>.pdf`).
fn side_name(d: &Path, lane: &str) -> PathBuf {
    let n = d.file_name().unwrap_or_default().to_string_lossy();
    d.with_file_name(
        n.rfind('.')
            .filter(|&i| i > 0 && i + 1 < n.len())
            .map_or_else(
                || format!("{n}-{lane}"),
                |i| format!("{}-{lane}.{}", &n[..i], &n[i + 1..]),
            ),
    )
}
/// Moves one git-ignored file of a lane to the same path in the project, never over what is there: an identical file there needs no move, a different one keeps its place and the lane's lands next to it with the lane's id in its name (`doc-<lane>.pdf`). The path it ends at.
pub(super) fn move_ignored(src: &Path, repo: &Path, rel: &Path, lane: &str) -> Option<PathBuf> {
    let dest = std::iter::successors(Some(repo.join(rel)), |d| Some(side_name(d, lane)))
        .find(|d| !d.exists() || std::fs::read(d).ok() == std::fs::read(src).ok())?;
    let Some(()) = (!dest.exists()).then_some(()) else {
        return Some(dest);
    };
    (
        dest.parent().map(std::fs::create_dir_all),
        std::fs::rename(src, &dest)
            .or_else(|_| std::fs::copy(src, &dest).and_then(|_| std::fs::remove_file(src)))
            .ok(),
    )
        .1
        .map(|_| dest)
}
pub(super) fn files_under(p: &Path, out: &mut Vec<PathBuf>) {
    std::fs::read_dir(p)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .for_each(|e| {
            (
                (e.is_symlink() || e.is_file()).then(|| out.push(e.clone())),
                (!e.is_symlink() && e.is_dir()).then(|| files_under(&e, out)),
            )
                .0
                .unwrap_or(())
        });
}
