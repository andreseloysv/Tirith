//! The daemon starts with what an app opened from the Dock has, however this app was opened. Opened from a
//! terminal (`open -a NeoBot`, or by an agent's session after a rebuild) the app holds that terminal's whole
//! environment: the keys its owner exports, an agent session's own variables. The daemon would hand all of it to
//! every agent it starts. Seen on 2026-11-25: a hundred variables instead of thirteen.

use std::process::Command;

/// What the daemon keeps of this process's environment on macOS.
#[cfg(target_os = "macos")]
const KEPT: [&str; 10] = ["HOME", "USER", "LOGNAME", "SHELL", "TMPDIR", "LANG", "SSH_AUTH_SOCK", "TIRITH_HOME", "TIRITH_PORT", "__CF_USER_TEXT_ENCODING"];

/// Gives `cmd` only that, and the system's own PATH: the daemon reads the login shell's PATH itself.
#[cfg(target_os = "macos")]
pub fn clean(cmd: &mut Command) -> &mut Command {
    cmd.env_clear();
    cmd.envs(KEPT.iter().filter_map(|name| Some((name, std::env::var_os(name)?))));
    cmd.env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
}

/// Elsewhere a desktop session's environment is what the daemon needs (its display, its session bus): kept whole.
#[cfg(not(target_os = "macos"))]
pub fn clean(cmd: &mut Command) -> &mut Command {
    cmd
}
