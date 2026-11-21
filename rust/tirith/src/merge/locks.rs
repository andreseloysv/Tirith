//! The locks of the merge queue: the merge slot, the main checks already run and one stage at a time per project.
use std::{collections::BTreeMap, sync::Arc, sync::Mutex};
/// Held while something uses a merge slot (a train or a main check).
pub(super) static BUSY: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
/// Main checks already run: (project, main sha, stage) → green.
pub(super) static MAIN_OK: Mutex<BTreeMap<(String, String, String), bool>> =
    Mutex::new(BTreeMap::new());
/// One lock per project: test stages of one project never run at the same time (servers on one port, one folder).
static STAGE_LOCKS: Mutex<BTreeMap<String, Arc<tokio::sync::Mutex<()>>>> =
    Mutex::new(BTreeMap::new());
pub(super) fn stage_lock(project: &str) -> Arc<tokio::sync::Mutex<()>> {
    STAGE_LOCKS
        .lock()
        .unwrap()
        .entry(project.to_string())
        .or_default()
        .clone()
}
