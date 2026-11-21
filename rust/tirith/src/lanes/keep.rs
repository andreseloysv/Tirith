//! Before a worktree goes: its git-ignored files that are no build cache move to the project, kept out of git there.
use super::exclude::ensure_excluded;
use super::ignored::{files_under, is_cache, move_ignored};
use crate::app::git;
use std::path::{Path, PathBuf};

/// The git-ignored files and folders at the top of what git lists, build caches left out.
async fn ignored_tops(ws: &Path) -> Vec<PathBuf> {
    let listed = git(
        ws,
        &[
            "ls-files",
            "--others",
            "--ignored",
            "--exclude-standard",
            "--directory",
            "-z",
        ],
    )
    .await
    .unwrap_or_default();
    listed
        .split('\0')
        .filter(|r| !r.is_empty() && !is_cache(Path::new(r.trim_end_matches('/'))))
        .map(|r| PathBuf::from(r.trim_end_matches('/')))
        .collect()
}
/// Moves the files of one top to the project; what to exclude there: the folder itself, or the paths a lone file landed at.
fn move_top(ws: &Path, repo: &Path, top: &Path, lane: &str) -> Vec<PathBuf> {
    let (src, is_dir, mut files) = (
        ws.join(top),
        ws.join(top).is_dir() && !ws.join(top).is_symlink(),
        Vec::<PathBuf>::new(),
    );
    (
        (src.is_symlink() || src.is_file()).then(|| files.push(src.clone())),
        is_dir.then(|| files_under(&src, &mut files)),
    )
        .0
        .unwrap_or(());
    let moved: Vec<PathBuf> = files
        .iter()
        .filter_map(|p| {
            p.strip_prefix(ws)
                .ok()
                .filter(|rel| !is_cache(rel))
                .and_then(|rel| move_ignored(p, repo, rel, lane))
        })
        .collect();
    if is_dir {
        (!moved.is_empty())
            .then(|| top.to_path_buf())
            .into_iter()
            .collect::<Vec<_>>()
    } else {
        moved
            .iter()
            .filter_map(|d| d.strip_prefix(repo).ok().map(Path::to_path_buf))
            .collect()
    }
}
/// Every ignored top of the worktree moves first, then each joins the project's excludes.
pub(super) async fn keep_ignored(ws: &Path, repo: &Path, lane: &str) {
    let mut targets = Vec::<PathBuf>::new();
    for top in &ignored_tops(ws).await {
        targets.extend(move_top(ws, repo, top, lane));
    }
    for rel in &targets {
        ensure_excluded(repo, rel).await;
    }
}
