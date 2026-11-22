//! With no service manager (a container, or only for now): the daemon in a session of its own, so it outlives the terminal.
use super::unit::Unit;
use crate::setup::daemon;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

pub(super) fn start(u: &Unit) -> Result<(), String> {
    let path = u.log()?;
    let failed = |e: std::io::Error| format!("{}: {e}", path.display());
    let mut open = std::fs::OpenOptions::new();
    let log = open.create(true).append(true).open(&path).map_err(failed)?;
    let err = log.try_clone().map_err(failed)?;
    let mut command = Command::new(&u.exe);
    command
        .arg("serve")
        .env("TIRITH_HOME", &u.home)
        .current_dir(&u.user_home)
        .stdin(Stdio::null())
        .stdout(log)
        .stderr(err);
    unsafe {
        command.pre_exec(|| {
            libc::setsid(); // (only async-signal-safe calls belong here)
            Ok(())
        });
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("cannot start {}: {e}", u.exe.display()))
}

/// Ends this home's daemon by the pid it gives; an agent's lanes go on and the next daemon takes them up.
pub(super) fn stop(u: &Unit) -> Result<(), String> {
    let Some(said) = daemon::answering() else {
        return Ok(());
    };
    daemon::is_of(&said, &u.home)
        .then_some(())
        .ok_or("the daemon on this port is not this home's: stop that one, or choose another port with --port")?;
    let pid = said["pid"]
        .as_i64()
        .filter(|p| *p > 1)
        .ok_or("the daemon gave no pid")?;
    let sent = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
    (sent == 0)
        .then_some(())
        .ok_or_else(|| format!("cannot stop pid {pid}: {}", std::io::Error::last_os_error()))?;
    daemon::down()
}
