//! `tirith net run`: the launcher of a confined process, the parent that stays outside and carries its few ports.
use super::{child::child, handover, relay, supervise};
use std::io::Write;
use std::net::TcpListener;
use std::os::fd::FromRawFd;
use std::os::unix::net::UnixStream;

/// Runs `program` in a network of its own in which only `ports` lead out, to the same ports of this machine's loopback; its exit code.
pub(super) fn run(ports: &[u16], hidden: &[String], program: &[String]) -> i32 {
    if ports.len() > handover::MOST {
        return said(&format!("at most {} ports lead out", handover::MOST));
    }
    let Ok((mut ours, theirs)) = UnixStream::pair() else {
        return said("no socket pair");
    };
    let me = unsafe { libc::getpid() };
    // Still one thread: the child may go on as a whole program, and unshare a user namespace.
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        return said("cannot fork");
    }
    if pid == 0 {
        drop(ours);
        child(theirs, ports, hidden, program, me);
    }
    drop(theirs);
    supervise::follow(pid);
    if let Some(fds) = handover::receive(&ours) {
        for (n, (fd, port)) in fds.into_iter().zip(ports).enumerate() {
            let inside = unsafe { TcpListener::from_raw_fd(fd) };
            relay::serve(inside, *port, n == 0); // the first is the daemon's (`confine.rs`)
        }
        let _ = ours.write_all(&[1]);
    }
    supervise::ended(pid)
}

fn said(why: &str) -> i32 {
    eprintln!("tirith sandbox: {why}");
    125
}
