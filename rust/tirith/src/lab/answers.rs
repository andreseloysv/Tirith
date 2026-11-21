//! A replay's tool calls answered from its capture, in Tirith's `PreToolUse` hook (`server/hook.rs`, Anthropic's and Muse's alike): a call that reads outside gets the next call of that tool in the capture, in order, as the reason of its refusal («Recorded result (replay): …»); one the capture does not have left is refused as not available in a replay, and so is one that would reach the owner. Local calls (its folder, its skills) run as always.
use super::outside::{outside, to_owner};
use crate::server::s;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A worktree a replay's turn runs in, as given and as the disk resolves it, with the capture's outside calls not answered yet.
struct Open {
    real: PathBuf,
    calls: Vec<Option<Value>>,
}

static OPEN: Mutex<BTreeMap<PathBuf, Open>> = Mutex::new(BTreeMap::new());

/// The replay's turn in `dir` starts: the capture's calls that read outside wait for it.
pub(super) fn open(dir: &Path, calls: &[Value]) {
    let out = calls.iter().filter(|c| {
        let (known, input) = crate::rules::normalize(&s(c, "tool"), &c["input"], "");
        outside(&known, &input)
    });
    let real = dir.canonicalize().unwrap_or(dir.to_path_buf());
    let calls = out.cloned().map(Some).collect();
    OPEN.lock()
        .unwrap()
        .insert(dir.to_path_buf(), Open { real, calls });
}

/// The replay's turn in `dir` ended.
pub(super) fn close(dir: &Path) {
    OPEN.lock().unwrap().remove(dir);
}

/// True for a path inside the worktree of a replay's turn or a shadow B run on its way: its bot writes there as in its own folder.
pub fn in_replay(path: &Path) -> bool {
    if super::shadow_open::inside(path) {
        return true;
    }
    let open = OPEN.lock().unwrap();
    open.iter()
        .any(|(dir, o)| path.starts_with(dir) || path.starts_with(&o.real))
}

/// The refusal a hook gives a replay's call from `cwd` (`tool` as its agent named it, `known` and `input` under the rules' names): its recorded result, or why it is not available; none for a call outside any replay, or a local one.
pub fn answer(cwd: &str, tool: &str, known: &str, input: &Value) -> Option<String> {
    let cwd = Path::new(cwd);
    let real = cwd.canonicalize().unwrap_or(cwd.to_path_buf());
    let mut open = OPEN.lock().unwrap();
    let here = |(dir, o): &(&PathBuf, &mut Open)| cwd.starts_with(dir) || real.starts_with(&o.real);
    let (_, o) = open.iter_mut().find(|e| here(e))?;
    if to_owner(known, input) {
        return Some(format!(
            "{tool}: not available in a replay — nothing of a replay reaches the owner"
        ));
    }
    outside(known, input).then_some(())?; // a local call runs as always
    let next = o
        .calls
        .iter_mut()
        .find(|c| c.as_ref().is_some_and(|c| c["tool"] == tool));
    Some(match next.and_then(Option::take) {
        Some(c) => format!("Recorded result (replay): {}", s(&c, "result")),
        None => format!("{tool}: not available in a replay (its capture has no such call left)"),
    })
}
