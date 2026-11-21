//! A shadow pair: its A (the real run) and its B (beside it) end in either order; once both have, B is compared with that A — the same task at the same time.
use super::store::Experiment;
use crate::app::Shared;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type Ends = (Option<i64>, Option<i64>);

/// The pairs with one side ended, by their experiment's worktree (a path, so two Tirith homes never share one) and pair: A's turn, B's turn.
static PAIRS: Mutex<BTreeMap<(PathBuf, i64), Ends>> = Mutex::new(BTreeMap::new());

/// One side of pair `pair` of experiment `e` (its worktree `dir`) ended (`ends` holds its turn); with both ended, B is compared with its A.
pub(super) fn done(app: &Shared, e: &Experiment, (dir, pair): (&Path, i64), ends: Ends) {
    let at = (dir.to_path_buf(), pair);
    let both = {
        let mut pairs = PAIRS.lock().unwrap();
        let p = pairs.entry(at.clone()).or_default();
        *p = (p.0.or(ends.0), p.1.or(ends.1));
        let both = p.0.zip(p.1);
        let _ = both.is_some().then(|| pairs.remove(&at));
        both
    };
    if let Some((a, b)) = both {
        super::compare::compare_pair(app, e, a, b);
    }
}
