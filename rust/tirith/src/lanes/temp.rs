//! A lane's own temporary folder, beside its worktree (`slot-N.tmp`). An agent keeps files outside the worktree:
//! its session's scratch files, the output of its background commands, a private build folder of a gigabyte. Left
//! in the system's temporary folder they outlive the lane, since nothing empties it. Where the catalog names the
//! agent's variable for that folder (`tmp_var`), a lane's agent gets this one instead, and it goes with the
//! worktree when the lane merges, stops or fails for good (`cleanup_lane`); the sweep takes one whose worktree
//! went while the daemon was down.
use crate::app::Shared;
use std::path::{Path, PathBuf};

const END: &str = ".tmp";

/// The temporary folder of the lane whose worktree is `ws`.
pub fn of(ws: &Path) -> PathBuf {
    let mut name = ws.file_name().unwrap_or_default().to_os_string();
    name.push(END);
    ws.with_file_name(name)
}

/// The variable that puts the temporary files of the agent whose command `program` is under the lane's folder;
/// none for an agent the catalog names no such variable for.
pub(super) fn env(app: &Shared, program: &str, ws: &Path) -> Vec<(String, String)> {
    let agents = app.cfg().factory.commands();
    let alias = agents.iter().find(|(_, command)| command == program);
    let known = alias.and_then(|(alias, _)| crate::setup::known_agent(alias));
    given(&known.map(|k| k.tmp_var).unwrap_or_default(), ws)
}

/// `var` naming the lane's folder, made first; nothing without a variable or where the folder cannot be made.
pub(super) fn given(var: &str, ws: &Path) -> Vec<(String, String)> {
    let dir = of(ws);
    if var.is_empty() || std::fs::create_dir_all(&dir).is_err() {
        return vec![];
    }
    vec![(var.to_string(), dir.display().to_string())]
}

/// The lane's temporary folder goes, with all it holds.
pub(super) fn remove(ws: &Path) {
    let _ = std::fs::remove_dir_all(of(ws));
}

/// In `dir` (a project's lanes), every temporary folder whose worktree is gone goes too.
pub(super) fn sweep(dir: &Path) {
    let entries = std::fs::read_dir(dir).into_iter().flatten().flatten();
    for path in entries.map(|e| e.path()) {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let slot = name.strip_suffix(END).filter(|s| s.starts_with("slot-"));
        if slot.is_some_and(|slot| !dir.join(slot).exists()) {
            let _ = std::fs::remove_dir_all(&path);
        }
    }
}
