//! What the broker opens when the daemon starts: its socket, and the rules' door for the limited ones.
use super::doors::Doors;
use crate::app::Shared;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::Path;

pub(super) fn open(app: &Shared, path: &Path) -> Result<(UnixListener, Doors), String> {
    let main = app.cfg().port; // as bound now: what a later change of it says moves no listener
    let rules = crate::server::rules_door(app)?; // none of them starts without it
    let _ = std::fs::remove_file(path); // left by a daemon before
    let listener = UnixListener::bind(path).map_err(|e| e.to_string())?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| e.to_string())?;
    Ok((listener, Doors { main, rules }))
}
