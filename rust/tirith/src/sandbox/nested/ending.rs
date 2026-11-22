//! Ending what the broker started. Codex gives each command a session of its own, so its process group is not all of it: every descendant goes.
use crate::app::kill_group;
use libc::pid_t;
use std::process::Child;

/// `child` and all below it, ended now.
pub(super) fn end(child: &mut Child) {
    end_all(child.id() as pid_t);
    let _ = child.wait();
}

/// Process `pid`, a group's leader, and all below it, ended now.
pub(super) fn end_all(pid: pid_t) {
    // SAFETY: kill only sends a signal; held still first, so it starts nothing while its children are counted.
    unsafe { libc::kill(pid, libc::SIGSTOP) };
    let mut below = vec![];
    collect(pid, &mut below);
    kill_group(pid as u32, libc::SIGKILL);
    for p in below {
        // SAFETY: as above, to a process counted a moment ago as this one's.
        unsafe { libc::kill(p, libc::SIGKILL) };
    }
}

fn collect(of: pid_t, all: &mut Vec<pid_t>) {
    let mut kids = [0 as pid_t; 1024];
    let size = std::mem::size_of_val(&kids) as libc::c_int;
    // SAFETY: the kernel writes no more than the buffer's size.
    unsafe { libc::proc_listchildpids(of, kids.as_mut_ptr().cast(), size) };
    for kid in kids.into_iter().filter(|p| *p > 1) {
        if !all.contains(&kid) && all.len() < 4096 {
            unsafe { libc::kill(kid, libc::SIGSTOP) };
            all.push(kid);
            collect(kid, all);
        }
    }
}
