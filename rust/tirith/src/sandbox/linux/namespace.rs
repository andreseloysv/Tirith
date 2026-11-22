//! A network of one's own: a user and a network namespace with only a loopback, reached by nothing and reaching nothing; and mounts of one's own (`hide.rs`).
use std::io::{Error, Write};

/// Moves this process (it must have one thread) into new namespaces, as the same user and group, its loopback up.
pub(super) fn enter() -> Result<(), String> {
    let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
    let flags = libc::CLONE_NEWUSER | libc::CLONE_NEWNET | libc::CLONE_NEWNS;
    if unsafe { libc::unshare(flags) } != 0 {
        return Err(failed("unshare"));
    }
    mapped(uid, gid)?;
    loopback_up()
}

/// root keeps its powers over its own mounts, so it goes one namespace further in, where they are locked.
pub(super) fn locked() -> Result<(), String> {
    let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
    if uid != 0 {
        return Ok(()); // it loses them as the program starts
    }
    if unsafe { libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNS) } != 0 {
        return Err(failed("unshare"));
    }
    mapped(uid, gid)
}

fn mapped(uid: u32, gid: u32) -> Result<(), String> {
    write("/proc/self/setgroups", "deny")?;
    write("/proc/self/uid_map", &format!("{uid} {uid} 1"))?;
    write("/proc/self/gid_map", &format!("{gid} {gid} 1"))
}

fn failed(what: &str) -> String {
    format!("{what}: {}", Error::last_os_error())
}

/// One write, as the id maps want it.
fn write(path: &str, text: &str) -> Result<(), String> {
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .and_then(|mut f| f.write_all(text.as_bytes()))
        .map_err(|e| format!("{path}: {e}"))
}

fn loopback_up() -> Result<(), String> {
    unsafe {
        let sock = libc::socket(libc::AF_INET, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0);
        if sock < 0 {
            return Err(failed("socket"));
        }
        let mut lo: libc::ifreq = std::mem::zeroed();
        for (to, from) in lo.ifr_name.iter_mut().zip(b"lo") {
            *to = *from as libc::c_char;
        }
        let up = libc::ioctl(sock, libc::SIOCGIFFLAGS as _, &mut lo) == 0 && {
            lo.ifr_ifru.ifru_flags |= libc::IFF_UP as libc::c_short;
            libc::ioctl(sock, libc::SIOCSIFFLAGS as _, &lo) == 0
        };
        let said = (!up).then(|| failed("loopback"));
        libc::close(sock);
        said.map_or(Ok(()), Err)
    }
}
