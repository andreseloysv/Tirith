//! Opening an address or a file the way the owner's desktop does.
use std::ffi::OsStr;
use std::process::{Command, Stdio};

/// Opens `what` (an address or a file) with the desktop's own handler: `open` on macOS, `xdg-open` elsewhere on Unix, the shell's URL handler on Windows (no `cmd /C start`, which would read an `&` in an address as a second command). True when it was handed over.
pub fn open(what: &OsStr) -> bool {
    let mut command = if cfg!(windows) {
        let mut c = Command::new("rundll32.exe");
        c.arg("url.dll,FileProtocolHandler");
        c
    } else if cfg!(target_os = "macos") {
        Command::new("open")
    } else {
        Command::new("xdg-open")
    };
    command
        .arg(what)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}
