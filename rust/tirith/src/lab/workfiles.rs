//! A bot's working files in an experiment's worktree. Git holds its memory, its skills and its tuning alone, and a routine's task needs the rest (its data, what it has seen): a live B run works on the bot's own, for it is a real run; a shadow or replayed one on a copy, so nothing it changes stays.
use std::fs;
use std::path::{Path, PathBuf};

/// A copy holds no more than this.
const MOST: u64 = 64 << 20;

/// A working file's name: not what git holds, not a draft, and nothing hidden (agents' settings and skills).
fn works(name: &str) -> bool {
    !name.starts_with('.') && !matches!(name, "memory.md" | "routines.toml" | "drafts")
}

/// Whether `dir` is a folder itself. The daemon runs outside the sandbox, so it works in no worktree that an agent swapped for a link.
fn itself(dir: &Path) -> bool {
    fs::symlink_metadata(dir).is_ok_and(|m| m.is_dir())
}

/// The working files at the top of `dir`.
fn entries(dir: &Path) -> Vec<(String, PathBuf)> {
    let all = fs::read_dir(dir).into_iter().flatten().flatten();
    all.filter_map(|e| Some((e.file_name().into_string().ok()?, e.path())))
        .filter(|(name, _)| works(name))
        .collect()
}

/// A live B run starts in `tree`: what works in `main` is there, as links to it.
pub(super) fn share(main: &Path, tree: &Path) {
    if !itself(tree) {
        return;
    }
    back(main, tree);
    for (name, path) in entries(main) {
        let at = tree.join(name);
        if fs::symlink_metadata(&at).is_err() {
            let _ = std::os::unix::fs::symlink(path, at);
        }
    }
}

/// What a live B run made, or wrote by replacing, is the bot's own.
pub(super) fn back(main: &Path, tree: &Path) {
    if !itself(tree) {
        return;
    }
    for (name, path) in entries(tree) {
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        let to = main.join(name);
        let free = fs::symlink_metadata(&to).map_or(true, |was| was.is_file() && meta.is_file());
        if free && !meta.is_symlink() {
            let _ = fs::rename(&path, &to);
        }
    }
}

/// A run whose changes must not stay starts in `tree`: what works in `main`, copied as it is now. Links are left out: they lead out of the copy.
pub(super) fn copied(main: &Path, tree: &Path) {
    if !itself(tree) {
        return;
    }
    for (_, path) in entries(tree) {
        let real_dir = fs::symlink_metadata(&path).is_ok_and(|m| m.is_dir());
        let _ = if real_dir {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
    }
    let mut room = MOST;
    for (name, path) in entries(main) {
        copy(&path, &tree.join(name), &mut room);
    }
}

fn copy(from: &Path, to: &Path, room: &mut u64) {
    let Ok(meta) = fs::symlink_metadata(from) else {
        return;
    };
    if meta.is_file() && meta.len() <= *room {
        *room -= meta.len();
        let _ = fs::copy(from, to);
    } else if meta.is_dir() && fs::create_dir(to).is_ok() {
        for inner in fs::read_dir(from).into_iter().flatten().flatten() {
            copy(&inner.path(), &to.join(inner.file_name()), room);
        }
    }
}
