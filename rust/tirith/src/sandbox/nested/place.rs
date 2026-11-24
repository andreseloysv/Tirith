//! The broker's socket: in the user's own temporary folder of macOS (0700, whatever TMPDIR says), one for each NeoBot home. Who calls is told by the kernel.
use sha2::{Digest, Sha256};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

const USER_TEMP: libc::c_int = 65537;
const SOL_LOCAL: libc::c_int = 0;
const PEER_PID: libc::c_int = 2;

pub(super) fn socket(home: &Path) -> Option<PathBuf> {
    let mut buf = [0u8; 256];
    // SAFETY: confstr writes at most the buffer's size and says how much the value needs.
    let n = unsafe { libc::confstr(USER_TEMP, buf.as_mut_ptr().cast(), buf.len()) };
    (n > 1 && n <= buf.len()).then_some(())?;
    let dir = std::str::from_utf8(&buf[..n - 1]).ok()?;
    let home = home.canonicalize().unwrap_or_else(|_| home.to_path_buf());
    let name: String = Sha256::digest(home.to_string_lossy().as_bytes())[..6]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Some(Path::new(dir).join(format!("tirith-{name}.sock")))
}

/// The other end's process, when it is this user's.
pub(super) fn peer(s: &UnixStream) -> Option<libc::pid_t> {
    let (mut uid, mut gid, mut pid): (libc::uid_t, libc::gid_t, libc::pid_t) = (0, 0, 0);
    let mut len = std::mem::size_of_val(&pid) as libc::socklen_t;
    // SAFETY: each out-parameter lives through its call and has the size given.
    unsafe {
        let mine =
            libc::getpeereid(s.as_raw_fd(), &mut uid, &mut gid) == 0 && uid == libc::geteuid();
        let known = libc::getsockopt(
            s.as_raw_fd(),
            SOL_LOCAL,
            PEER_PID,
            (&raw mut pid).cast(),
            &mut len,
        ) == 0;
        (mine && known && pid > 0).then_some(pid)
    }
}
