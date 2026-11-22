//! The gateways open now, each a bot's run's: the daemon holds its connectors' connections, and only the caller that opened one finds it again. One whose gateway process is gone is closed at the next open, and its connectors with it.
use super::link::Link;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

pub struct Session {
    /// The bot's page id and its key (its role and conversation).
    pub bot: String,
    pub key: String,
    /// Whom its token is: the proxy and the connectors' processes run as this.
    pub who: String,
    /// The gateway process, while it lives.
    pub pid: u32,
    pub links: Vec<Link>,
}

static OPEN: LazyLock<Mutex<HashMap<String, Arc<Session>>>> = LazyLock::new(Default::default);

/// Keeps `s` open under a new id, first closing those whose gateway is gone.
pub fn keep(s: Session) -> String {
    let id = crate::rooms::sealed::sha_hex(&rand::random::<[u8; 16]>())[..24].to_string();
    let mut open = OPEN.lock().unwrap();
    open.retain(|_, s| s.pid == 0 || crate::app::alive(s.pid));
    open.insert(id.clone(), Arc::new(s));
    id
}

/// The session `id`, for the caller `who` that opened it alone.
pub fn find(id: &str, who: &str) -> Option<Arc<Session>> {
    let open = OPEN.lock().unwrap();
    open.get(id).filter(|s| s.who == who).cloned()
}

/// Closes session `id` of `who`: its connectors stop once no call holds them.
pub fn close(id: &str, who: &str) -> bool {
    let mut open = OPEN.lock().unwrap();
    let theirs = open.get(id).is_some_and(|s| s.who == who);
    theirs && open.remove(id).is_some()
}
