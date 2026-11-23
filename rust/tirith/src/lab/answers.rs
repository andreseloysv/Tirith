//! A replay's tool calls answered from its capture, in OpenBot's `PreToolUse` hook (`server/hook.rs`, Anthropic's and Muse's alike): a call that reads outside gets the capture's call that asked for the same thing (`call_key.rs`: the tool and its URL, query or command), as the reason of its refusal («Recorded result (replay): …»); one the capture has no such call for is refused as not available in a replay, and counted as missed (the pair proves nothing, `replay.rs`); so is one that would reach the owner refused. Local calls (its folder, its skills) run as always.
use super::call_key::key;
use super::outside::{outside, to_owner};
use super::recorded::Call;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A worktree a replay's turn runs in, as given and as the disk resolves it, with the capture's outside calls and how many calls of the replay found none.
struct Open {
    real: PathBuf,
    calls: Vec<Call>,
    missed: usize,
    heard: usize,
}

static OPEN: Mutex<BTreeMap<PathBuf, Open>> = Mutex::new(BTreeMap::new());

/// The replay's turn in `dir` starts: the capture's calls that read outside wait for it.
pub(super) fn open(dir: &Path, calls: &[Value]) {
    let calls = super::recorded::reads(calls);
    let real = dir.canonicalize().unwrap_or(dir.to_path_buf());
    let open = Open {
        real,
        calls,
        missed: 0,
        heard: 0,
    };
    OPEN.lock().unwrap().insert(dir.to_path_buf(), open);
}

/// The replay's turn in `dir` ended: how many of its outside calls the capture had no answer for, and how many of its calls the hook heard.
pub(super) fn close(dir: &Path) -> (usize, usize) {
    let open = OPEN.lock().unwrap().remove(dir);
    open.map_or((0, 0), |o| (o.missed, o.heard))
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
    let (dir, o) = open.iter_mut().find(|e| here(e))?;
    o.heard += 1;
    if to_owner(known, input) {
        return Some(format!(
            "{tool}: not available in a replay — nothing of a replay reaches the owner"
        ));
    }
    if !outside(known, input) {
        // a local call runs as always; one that changes something beyond the worktree never does
        let acts = |at: &Path| super::outward::outward(known, input, at);
        return (acts(dir) && acts(&o.real)).then(|| {
            format!("{tool}: not available in a replay — it changes something outside its folder")
        });
    }
    let asked = key(known, input);
    let same = |c: &&mut Call| !c.used && c.tool == tool && c.key == asked;
    let found = o
        .calls
        .iter_mut()
        .find(same)
        .map(|c| (c.used = true, c.result.clone()).1);
    let _ = found.is_none().then(|| o.missed += 1);
    Some(match found {
        Some(result) => format!("Recorded result (replay): {result}"),
        None => format!(
            "{tool}: not available in a replay (its capture has no call that asked for this)"
        ),
    })
}
