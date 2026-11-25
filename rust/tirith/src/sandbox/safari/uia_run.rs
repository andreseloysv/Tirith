//! Running the fixed script of `uia.rs` (assets/win-browser.ps1) for one command, out of sight, on Windows.
#[cfg(windows)]
use super::uia::answer;
use super::uia::{browser, vars};
use serde_json::Value;
use std::path::Path;

/// The fixed script.
const SCRIPT: &str = include_str!("../../../assets/win-browser.ps1");
/// PowerShell runs the script's text as a block: a script file would need the machine's policy to allow it.
#[cfg(windows)]
const RUN: &str =
    "& ([ScriptBlock]::Create([IO.File]::ReadAllText($env:NEOBOT_SCRIPT, [Text.Encoding]::UTF8)))";

/// Runs the script for one command of `home`'s daemon.
#[cfg(windows)]
pub async fn run(home: &Path, which: &str, b: &Value) -> Result<String, String> {
    let (process, name) = browser(which).ok_or("chrome or edge")?;
    let vars = vars(process, b)?;
    let file = home.join("win-browser.ps1");
    if std::fs::read(&file).ok().as_deref() != Some(SCRIPT.as_bytes()) {
        std::fs::write(&file, SCRIPT).map_err(|e| format!("{}: {e}", file.display()))?;
    }
    let mut cmd = tokio::process::Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", RUN]);
    cmd.env("NEOBOT_SCRIPT", &file).envs(vars);
    cmd.stdin(std::process::Stdio::null()).kill_on_drop(true);
    let work = crate::os::own_group(&mut cmd).output();
    let out = tokio::time::timeout(std::time::Duration::from_secs(90), work).await;
    let out = out
        .map_err(|_| format!("{name}: no answer in 90 seconds"))?
        .map_err(|e| format!("{name}: {e}"))?;
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).to_string();
    answer(
        name,
        out.status.success(),
        &text(&out.stdout),
        &text(&out.stderr),
    )
}

/// Elsewhere there is no UI Automation: the owner's own browser on a Mac is Safari.
#[cfg(not(windows))]
pub async fn run(_home: &Path, which: &str, b: &Value) -> Result<String, String> {
    let (process, name) = browser(which).ok_or("chrome or edge")?;
    vars(process, b)?;
    let _ = SCRIPT;
    Err(format!("{name} is driven only on Windows; on a Mac the owner's own browser is Safari (neobot safari …)"))
}
