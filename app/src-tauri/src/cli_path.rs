//! `neobot` in the owner's terminal on Windows: the folder that holds the app's neobot.exe joins the
//! user's PATH, once, so `neobot setup` runs in any new PowerShell. macOS and Linux link it instead (`cli_link`).

use std::path::Path;
use std::process::Command;

/// Makes `tirith.exe` resolve beside `neobot.exe`, as a hard link made once (a copy when the link fails).
pub fn alias(dir: &Path) {
    let (from, to) = (dir.join("neobot.exe"), dir.join("tirith.exe"));
    if from.is_file() && !to.exists() {
        let _ = std::fs::hard_link(&from, &to).or_else(|_| std::fs::copy(&from, &to).map(drop));
    }
}

/// Adds `dir` to the user's PATH (HKCU\Environment, which new terminals read) when this process's PATH lacks it.
pub fn add(dir: &Path) -> std::io::Result<()> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    if std::env::split_paths(&path).any(|d| d == dir) {
        return Ok(());
    }
    // The folder goes in by the environment, never inside the script's text.
    let script = "$d = $env:NEOBOT_DIR; $p = '' + [Environment]::GetEnvironmentVariable('Path', 'User'); \
        if (($p -split ';') -notcontains $d) { [Environment]::SetEnvironmentVariable('Path', ($p.TrimEnd(';') + ';' + $d).TrimStart(';'), 'User') }";
    let mut cmd = Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]).env("NEOBOT_DIR", dir);
    let done = crate::daemon::quiet(&mut cmd).status()?;
    done.success().then_some(()).ok_or_else(|| std::io::Error::other("powershell could not change PATH"))
}
