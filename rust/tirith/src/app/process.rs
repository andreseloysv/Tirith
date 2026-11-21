//! Signalling processes.

/// Signals a process group; pid 0 would be the daemon's own group, so it is never signalled.
pub fn kill_group(pid: u32, signal: i32) {
    let _ = (pid != 0).then(|| unsafe { libc::kill(-(pid as i32), signal) });
}

pub fn alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}
