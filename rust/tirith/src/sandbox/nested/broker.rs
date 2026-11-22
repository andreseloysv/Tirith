//! The daemon's side: it starts what a caller under Tirith's own profile asks, under one at least as tight, waits, and gives its exit code. Anyone else gets nothing: it is no way out of another sandbox. A caller that is gone takes it along.
use super::{around::around, place, spawn::spawn, Request};
use crate::app::{kill_group, Shared};
use crate::sandbox::{handover, mac};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::time::Duration;

/// The most a request holds (its environment is in it).
const MOST: u64 = 4 << 20;

pub(in crate::sandbox) fn start(app: &Shared) {
    if super::super::profile::enabled(app).is_err() {
        return; // no sandbox to nest in
    }
    let Some(path) = place::socket(&app.home) else {
        return;
    };
    let _ = std::fs::remove_file(&path); // left by a daemon before
    let listener = UnixListener::bind(&path).and_then(|l| {
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        Ok(l)
    });
    let listener = match listener {
        Ok(l) => l,
        Err(e) => {
            app.event(
                "",
                "",
                "error",
                "sandbox",
                &format!("Codex cannot run inside the sandbox ({e})"),
                "",
            );
            return;
        }
    };
    let app = app.clone();
    std::thread::spawn(move || {
        for caller in listener.incoming().flatten() {
            let app = app.clone();
            std::thread::spawn(move || serve(&app, caller));
        }
    });
}

fn serve(app: &Shared, mut caller: UnixStream) {
    let code = match answer(app, &caller) {
        Ok(code) => format!("{code}\n"),
        Err(why) => format!("{}\n", why.replace('\n', " ")),
    };
    let _ = caller.write_all(code.as_bytes());
}

fn answer(app: &Shared, caller: &UnixStream) -> Result<i32, String> {
    let pid = place::peer(caller).ok_or("only this user's processes")?;
    let _ = caller.set_read_timeout(Some(Duration::from_secs(10)));
    let fds = handover::receive(caller).unwrap_or_default();
    // SAFETY: each descriptor was just received and has no other owner.
    let io: Vec<OwnedFd> = fds
        .into_iter()
        .map(|fd| unsafe { OwnedFd::from_raw_fd(fd) })
        .collect();
    if !mac::ours(pid) || mac::tighter(pid) {
        return Err("only what runs under Tirith's own profile asks for a limit in it".into());
    }
    let mut line = String::new();
    BufReader::new(caller.take(MOST))
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    let asked: Request = serde_json::from_str(&line).map_err(|e| e.to_string())?;
    let io: [OwnedFd; 3] = io
        .try_into()
        .map_err(|_| "its stdin, stdout and stderr are missing")?;
    let (r, around) = around(app, asked)?;
    let mut child = spawn(&r, &around, io)?;
    let _ = caller.set_nonblocking(true);
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            kill_group(child.id(), libc::SIGTERM); // nothing it left outlives it
            let _ = caller.set_nonblocking(false);
            use std::os::unix::process::ExitStatusExt;
            return Ok(status.code().unwrap_or(128 + status.signal().unwrap_or(0)));
        }
        if matches!((&mut &*caller).read(&mut [0u8; 8]), Ok(0)) {
            super::ending::end(&mut child);
            return Err("its caller is gone".into());
        }
        std::thread::sleep(Duration::from_millis(150));
    }
}
