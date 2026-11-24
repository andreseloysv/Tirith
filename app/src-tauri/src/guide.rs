//! On Windows the app is how OpenBot gets installed, and the guide (`tirith setup`) is how agents get
//! connected: while config.toml connects none, the app opens the guide in a PowerShell window of its own.

use std::os::windows::process::CommandExt;
use std::path::Path;

/// Whether config.toml connects an agent (an `[agents.<alias>]` table).
fn connected(home: &Path) -> bool {
    std::fs::read_to_string(home.join("config.toml"))
        .ok()
        .and_then(|text| text.parse::<toml::Table>().ok())
        .and_then(|cfg| cfg.get("agents")?.as_table().map(|a| !a.is_empty()))
        .unwrap_or(false)
}

/// Opens the guide in a new console window when no agent is connected; the window stays open after it ends.
pub fn offer(home: &Path, tirith: &Path) {
    const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
    if connected(home) {
        return;
    }
    // The binary's path goes in by the environment, never inside the command's text.
    let _ = std::process::Command::new("powershell.exe")
        .args(["-NoExit", "-NoProfile", "-Command", "& $env:OPENBOT_TIRITH setup"])
        .env("OPENBOT_TIRITH", tirith)
        .creation_flags(CREATE_NEW_CONSOLE)
        .spawn();
}
