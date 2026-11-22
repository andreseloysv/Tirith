// How the checks call a test daemon: as the owner, or as an agent with its token.
use crate::app::{App, Shared};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

pub const HEADER: &str = "x-tirith-token";
static SERVED: LazyLock<Mutex<HashMap<u16, String>>> = LazyLock::new(Default::default);

/// Notes which home answers on `port`.
pub fn serving(port: u16, app: &Shared) {
    SERVED.lock().unwrap().insert(port, app.keys.owner.clone());
}

/// The owner's token of the test daemon on `port`.
pub fn owner(port: u16) -> String {
    SERVED
        .lock()
        .unwrap()
        .get(&port)
        .cloned()
        .unwrap_or_default()
}

/// The token `sandbox::wrap` gives `who`.
pub fn token(app: &App, who: &str) -> String {
    crate::sandbox::issued(app, who)
}
