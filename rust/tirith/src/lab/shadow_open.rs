//! The worktrees shadow B runs are in now: the task-local that makes the worktree every folder of the bot for its turn, and the refusal NeoBot's `PreToolUse` hook gives an outward action from one of them.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

tokio::task_local! {
    pub(super) static SHADOW: (PathBuf, String);
}

/// The worktrees of the B runs on their way, as given, with the disk's path for them.
static OPEN: Mutex<BTreeMap<PathBuf, PathBuf>> = Mutex::new(BTreeMap::new());

/// Inside a shadow B run, the worktree every folder of the bot resolves to.
pub(super) fn dir() -> Option<PathBuf> {
    SHADOW.try_with(|d| d.0.clone()).ok()
}

/// A B run starts in `dir`; false when one is on its way there already.
pub(super) fn open(dir: &Path) -> bool {
    let real = dir.canonicalize().unwrap_or(dir.to_path_buf());
    let mut open = OPEN.lock().unwrap();
    let free = !open.contains_key(dir);
    let _ = free.then(|| open.insert(dir.to_path_buf(), real));
    free
}

/// The B run in `dir` ended.
pub(super) fn close(dir: &Path) {
    OPEN.lock().unwrap().remove(dir);
}

/// True while a B run is on its way in the worktree `dir` (its experiment's: a path, so two NeoBot homes never share one).
pub(super) fn running(dir: &Path) -> bool {
    OPEN.lock().unwrap().contains_key(dir)
}

/// The worktree (as given) of the B run that `path` is in, if any.
fn holding(path: &Path) -> Option<PathBuf> {
    let real = path.canonicalize().unwrap_or(path.to_path_buf());
    let open = OPEN.lock().unwrap();
    let mut it = open.iter();
    let hit = it.find(|(d, r)| path.starts_with(d) || real.starts_with(r) || path.starts_with(r));
    hit.map(|(d, _)| d.clone())
}

/// True for a path inside the worktree of a shadow B run on its way.
pub fn inside(path: &Path) -> bool {
    holding(path).is_some()
}

/// The refusal a hook gives a call from `cwd` (`tool` as its agent named it, `known` and `input` under the rules' names) inside a shadow B run when it acts outward; none for any other call.
pub fn refused(cwd: &str, tool: &str, known: &str, input: &serde_json::Value) -> Option<String> {
    let dir = holding(Path::new(cwd)).filter(|_| !cwd.is_empty())?;
    let out = super::outward::outward(known, input, &dir)
        || super::outward_browser::acts(tool, input)
        || super::outward_browser::acts(known, input);
    out.then(|| {
        format!("{tool}: not available in a shadow run — it may read, but nothing it sends, buys, deletes or publishes leaves NeoBot; its answer only goes to the experiment")
    })
}

/// Inside a shadow B run, its worktree and the routine it runs.
pub(super) fn run() -> Option<(PathBuf, String)> {
    SHADOW.try_with(|d| d.clone()).ok()
}
