//! What the guide has root do through sudo: the system's own programs alone, looked up where no agent can write, and a file's text handed over on stdin, never copied from a folder agents write in.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Where a system keeps its own programs (NixOS's sudo in the last).
const SYSTEM: [&str; 5] = [
    "/usr/bin",
    "/bin",
    "/usr/sbin",
    "/sbin",
    "/run/wrappers/bin",
];
/// Root's part: the file made beside its place, then moved there in one step.
const PUT: &str = r#"set -e; umask 022; t=$(mktemp "$1.XXXXXX"); cat > "$t"; test -s "$t"; chmod 644 "$t"; mv -f "$t" "$1""#;

/// The system's own program `name`.
pub fn program(name: &str) -> Option<PathBuf> {
    SYSTEM
        .iter()
        .map(|d| Path::new(d).join(name))
        .find(|p| p.is_file())
}

/// The system's sudo.
pub fn sudo() -> Option<PathBuf> {
    program("sudo")
}

/// `to` holds `text`, root's and readable by all; `sudo` asks for the password on the terminal.
pub fn write(sudo: &Path, to: &Path, text: &str) -> bool {
    let started = Command::new(sudo)
        .args(["/bin/sh", "-c", PUT, "sh"])
        .arg(to)
        .stdin(Stdio::piped())
        .spawn();
    let Ok(mut child) = started else {
        return false;
    };
    let handed = child
        .stdin
        .take()
        .is_some_and(|mut stdin| stdin.write_all(text.as_bytes()).is_ok());
    let ended = child.wait().is_ok_and(|s| s.success());
    handed && ended
}
