//! Signalling processes.

/// The group that `pid` leads, as `kill` takes it: never 0 (the daemon's own), nor 1 (-1 is every process of the user), nor a number that does not fit.
pub(super) fn group(pid: u32) -> Option<i32> {
    i32::try_from(pid).ok().filter(|p| *p > 1).map(|p| -p)
}

/// Signals the process group that `pid` leads, when it may be one (`group`).
pub fn kill_group(pid: u32, signal: i32) {
    let _ = group(pid).map(|to| unsafe { libc::kill(to, signal) });
}

pub fn alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}
