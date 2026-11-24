//! Signalling processes: a process group on Unix; on Windows the tree a process heads.

/// What ends a process at once, and what asks it to end. Windows numbers them as Unix does; only `KILL` forces there.
#[cfg(unix)]
pub const KILL: i32 = libc::SIGKILL;
#[cfg(unix)]
pub const TERM: i32 = libc::SIGTERM;
#[cfg(windows)]
pub const KILL: i32 = 9;
#[cfg(windows)]
pub const TERM: i32 = 15;

/// The group that `pid` leads, as `kill` takes it: never 0 (the daemon's own), nor 1 (-1 is every process of the user), nor a number that does not fit.
#[cfg_attr(windows, allow(dead_code))]
pub(super) fn group(pid: u32) -> Option<i32> {
    i32::try_from(pid).ok().filter(|p| *p > 1).map(|p| -p)
}

/// Signals the process group that `pid` leads, when it may be one (`group`).
#[cfg(unix)]
pub fn kill_group(pid: u32, signal: i32) {
    let _ = group(pid).map(|to| unsafe { libc::kill(to, signal) });
}

#[cfg(unix)]
pub fn alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

/// `pid` and every process below it end (`taskkill /T`), forced for `KILL`; a polite `TERM` reaches only what has a window to close. Never the idle process (0) or the system's (4).
#[cfg(windows)]
pub fn kill_group(pid: u32, signal: i32) {
    use std::process::Stdio;
    if pid <= 4 {
        return;
    }
    let mut taskkill = std::process::Command::new("taskkill");
    taskkill.args(["/PID", &pid.to_string(), "/T"]);
    if signal == KILL {
        taskkill.arg("/F");
    }
    let quiet = taskkill.stdout(Stdio::null()).stderr(Stdio::null());
    let _ = quiet.stdin(Stdio::null()).status();
}

#[cfg(windows)]
pub fn alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    // SAFETY: the handle OpenProcess gives is closed once, and the code is written into a local.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut code = 0u32;
        let read = GetExitCodeProcess(handle, &mut code) != 0;
        CloseHandle(handle);
        read && code == STILL_ACTIVE as u32
    }
}
