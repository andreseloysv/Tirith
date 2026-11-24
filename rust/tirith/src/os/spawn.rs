//! Starting and ending children: a group of their own, a daemon that outlives its terminal, how a child ended.

/// A child in a group of its own, so a stop ends all it started: a process group on Unix; on Windows a new process group without a console window of its own (a daemon with no console would flash one for every child).
#[cfg(unix)]
pub fn own_group(command: &mut tokio::process::Command) -> &mut tokio::process::Command {
    command.process_group(0)
}

#[cfg(windows)]
pub fn own_group(command: &mut tokio::process::Command) -> &mut tokio::process::Command {
    use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW};
    command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW)
}

/// A daemon that outlives the terminal that starts it: a session of its own on Unix; on Windows a hidden console of its own, which its children share.
#[cfg(unix)]
pub fn detach(command: &mut std::process::Command) -> &mut std::process::Command {
    use std::os::unix::process::CommandExt;
    // SAFETY: setsid is async-signal-safe, the only kind of call that belongs between fork and exec.
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        })
    }
}

#[cfg(windows)]
pub fn detach(command: &mut std::process::Command) -> &mut std::process::Command {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::Foundation::{SetHandleInformation, HANDLE_FLAG_INHERIT};
    use windows_sys::Win32::System::Console::{
        GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };
    use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW};
    // A child gets every inheritable handle: a daemon holding this process's output pipe would keep whoever reads
    // that pipe (`tirith setup | …`, a captured run) waiting. Its own stdio stays inheritable: Rust duplicates it.
    for id in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
        // SAFETY: this process's own standard handle; only its inherit flag changes.
        unsafe { SetHandleInformation(GetStdHandle(id), HANDLE_FLAG_INHERIT, 0) };
    }
    command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW)
}

/// The signal that ended a child (Unix); Windows ends a process with a code alone.
#[cfg(unix)]
pub fn signal_of(status: &std::process::ExitStatus) -> Option<i32> {
    std::os::unix::process::ExitStatusExt::signal(status)
}

#[cfg(not(unix))]
pub fn signal_of(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}

/// This process's parent (Unix); Windows does not say it without walking every process.
#[cfg(unix)]
pub fn parent_id() -> Option<u32> {
    Some(std::os::unix::process::parent_id())
}

#[cfg(not(unix))]
pub fn parent_id() -> Option<u32> {
    None
}

/// Ends `pid` alone, not what it started: SIGTERM on Unix; on Windows a forced end, since a process without a window takes no polite one.
#[cfg(unix)]
pub fn terminate(pid: u32) -> std::io::Result<()> {
    let sent = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
    (sent == 0)
        .then_some(())
        .ok_or_else(std::io::Error::last_os_error)
}

#[cfg(windows)]
pub fn terminate(pid: u32) -> std::io::Result<()> {
    let ended = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    ended
        .success()
        .then_some(())
        .ok_or_else(|| std::io::Error::other(format!("taskkill /PID {pid} failed")))
}
