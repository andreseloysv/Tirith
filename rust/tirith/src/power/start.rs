//! Keeping the machine awake from the daemon's start.
use super::argv::argv;
use crate::app::Shared;
use std::process::Stdio;

/// Starts `caffeinate` on the daemon's own pid, so it also ends when the daemon is killed. Its end while the daemon lives is an error event: the machine may sleep from then on.
pub fn start(app: Shared) {
    let command = argv(
        &app.cfg().raw,
        cfg!(target_os = "macos"),
        std::process::id(),
    );
    let Some(argv) = command else { return };
    tokio::spawn(async move {
        let ended = tokio::process::Command::new(&argv[0])
            .args(&argv[1..])
            .stdin(Stdio::null())
            .status()
            .await;
        let why = ended.map_or_else(|e| e.to_string(), |status| status.to_string());
        let said = format!("keep_awake is off until a restart, caffeinate ended: {why}");
        app.event("", "", "error", "power", &said, "");
    });
}
