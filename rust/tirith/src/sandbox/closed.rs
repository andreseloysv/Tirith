//! Server mode (`[server] mode = "server"` in config.toml) fails closed: with no network sandbox no lane, no bot and no check starts, instead of running unconfined with a `net-off` event. And what only a Mac has says so on a Linux server.
use super::profile::{enabled, sandboxed};
use crate::app::Shared;
use crate::config::get;

/// This home is a server's.
pub fn server_mode(app: &Shared) -> bool {
    get::<String>(&app.cfg().raw, &["server", "mode"]).as_deref() == Some("server")
}

/// Why nothing may start for `who` now: server mode, and the network sandbox is not there.
pub fn refusal(app: &Shared, who: &str) -> Option<String> {
    (server_mode(app) && !sandboxed(app)).then(|| {
        let why = enabled(app).err().unwrap_or_else(|| "it is not available".into());
        format!("server mode: {who} does not start, the network sandbox is off ({why}). Turn it on (`neobot doctor` says how), or take [server] mode out of config.toml")
    })
}

/// Err when `what` is a Mac's alone (Safari, the owner's own browser, the folder picker) and this is a Linux server.
pub fn needs_mac(what: &str) -> Result<(), String> {
    mac_only(what, !cfg!(target_os = "linux"))
}

pub(super) fn mac_only(what: &str, on_a_mac: bool) -> Result<(), String> {
    on_a_mac
        .then_some(())
        .ok_or_else(|| format!("this needs the Mac: {what} is not on a Linux server"))
}

#[cfg(test)]
#[path = "../../checks/closed.rs"]
mod tests;
