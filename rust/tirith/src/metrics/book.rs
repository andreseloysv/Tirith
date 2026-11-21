//! What the metrics keep between computations: the last result the page gets, since when each project is idle, and the turn roles whose pass runs now.
use serde_json::Value;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
#[derive(Default)]
pub(super) struct Book {
    pub(super) last: Value,
    pub(super) idle_since: HashMap<String, u64>,
}
pub(super) static BOOK: LazyLock<Mutex<Book>> = LazyLock::new(Mutex::default);
/// The turn roles whose pass runs now: one pass of a role at a time.
pub(super) static RUNNING: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());
