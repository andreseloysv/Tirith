//! The shared state itself: what every module reaches through `Shared`.
use crate::config::{Config, Manifest};
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::SystemTime;
use tokio::sync::{broadcast, Notify};

pub type Shared = Arc<App>;

pub struct App {
    pub home: PathBuf,
    /// When this daemon started, in seconds since the epoch (an `Instant` stands still while the Mac sleeps).
    pub started: u64,
    pub cfg: RwLock<Arc<Config>>,
    pub db: Mutex<Connection>,
    pub lanes: Mutex<BTreeMap<String, crate::app::Lane>>,
    pub tx: broadcast::Sender<String>,
    pub plan_dirty: Mutex<BTreeSet<String>>,
    pub plan_lock: tokio::sync::Mutex<()>,
    pub sched_wake: Notify,
    pub merge_wake: Notify,
    pub children: Mutex<HashSet<String>>,
    pub(super) seen: Seen,
}

/// What was last read from disk, so a file is re-read only when it changed.
pub(super) struct Seen {
    /// Each project's `tirith.toml`: its mtime when read and what it said.
    pub manifests: Mutex<HashMap<PathBuf, (Option<SystemTime>, Manifest)>>,
    /// The last-seen changes of (config.toml, the factory folder).
    pub cfg_mtime: Mutex<(Option<SystemTime>, Option<SystemTime>)>,
}
