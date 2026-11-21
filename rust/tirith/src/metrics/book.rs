//! What the metrics keep between computations: the last result the page gets, since when each project is idle, and whether a review runs.
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{LazyLock, Mutex};
#[derive(Default)]
pub(super) struct Book {
    pub(super) last: Value,
    pub(super) idle_since: HashMap<String, u64>,
}
pub(super) static BOOK: LazyLock<Mutex<Book>> = LazyLock::new(Mutex::default);
pub(super) static REVIEWING: AtomicBool = AtomicBool::new(false);
