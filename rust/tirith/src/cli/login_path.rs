//! The PATH of the owner's login shell, for a daemon a desktop app started.

/// Started by the desktop app, the daemon has launchd's short PATH and its checks find no node, npm or brew tools. The PATH of the owner's login shell goes first, so a check runs as it does in the owner's terminal.
pub(super) fn use_login_path() {
    if cfg!(windows) {
        return; // no login shell there: the daemon keeps the PATH it was started with
    }
    let usual = if cfg!(target_os = "macos") {
        "/bin/zsh"
    } else {
        "/bin/sh"
    };
    let out = std::process::Command::new(std::env::var("SHELL").unwrap_or(usual.into()))
        .args(["-lc", "printf '\\n%s' \"$PATH\""])
        .output();
    let login = out
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .last()
                .unwrap_or("")
                .trim()
                .to_string()
        })
        .unwrap_or_default();
    let _ = (!login.is_empty()).then(|| {
        std::env::set_var(
            "PATH",
            format!("{login}:{}", std::env::var("PATH").unwrap_or_default()),
        )
    });
}
