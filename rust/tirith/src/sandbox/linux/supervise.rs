//! The launcher waits for the confined process: signals pass on to it, and its exit is the launcher's.
use std::sync::atomic::{AtomicI32, Ordering};

static CHILD: AtomicI32 = AtomicI32::new(0);

extern "C" fn pass_on(signal: libc::c_int) {
    let child = CHILD.load(Ordering::Relaxed);
    if child > 0 {
        unsafe { libc::kill(child, signal) };
    }
}

/// From now on what stops the launcher stops `child` instead (a kill of the group reaches both by itself).
pub(super) fn follow(child: i32) {
    CHILD.store(child, Ordering::Relaxed);
    let handler = pass_on as extern "C" fn(libc::c_int) as libc::sighandler_t;
    for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP, libc::SIGQUIT] {
        unsafe { libc::signal(signal, handler) };
    }
}

/// The exit code of `child` once it ended (128 + the signal that ended it).
pub(super) fn ended(child: i32) -> i32 {
    let mut code = 0;
    loop {
        if unsafe { libc::waitpid(child, &mut code, 0) } == child {
            break;
        }
        if std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
            return 125;
        }
    }
    CHILD.store(0, Ordering::Relaxed);
    if libc::WIFEXITED(code) {
        libc::WEXITSTATUS(code)
    } else {
        128 + libc::WTERMSIG(code)
    }
}
