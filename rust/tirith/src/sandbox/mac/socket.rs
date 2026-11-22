//! macOS: which of this user's processes hold one end of a loopback TCP connection (libproc, as `lsof` reads it).
use libc::{c_int, c_void, pid_t, proc_fdinfo};
use std::mem::size_of;

const UID_ONLY: u32 = 4;
const LIST_FDS: c_int = 1;
const SOCKET_INFO: c_int = 3;
const A_SOCKET: u32 = 2;
const TCP: i32 = 2;
/// `struct socket_fdinfo` of <sys/proc_info.h>, the same on arm64 and x86_64: its size and the fields read here.
const SIZE: usize = 792;
const KIND: usize = 256;
const THEIR_PORT: usize = 264;
const OWN_PORT: usize = 268;

/// The processes whose socket goes from port `from` to port `to`.
pub(super) fn holders(from: u16, to: u16) -> Vec<pid_t> {
    pids()
        .into_iter()
        .filter(|pid| {
            sockets(*pid)
                .into_iter()
                .any(|fd| ports(*pid, fd) == Some((from, to)))
        })
        .collect()
}

fn pids() -> Vec<pid_t> {
    let mut all = vec![0 as pid_t; 16384];
    let size = (all.len() * size_of::<pid_t>()) as c_int;
    // SAFETY: the buffer holds `size` bytes; the kernel writes no more and says how many.
    let got =
        unsafe { libc::proc_listpids(UID_ONLY, libc::geteuid(), all.as_mut_ptr().cast(), size) };
    all.truncate(got.max(0) as usize / size_of::<pid_t>());
    all.retain(|pid| *pid > 0);
    all
}

fn sockets(pid: pid_t) -> Vec<i32> {
    let each = size_of::<proc_fdinfo>();
    // SAFETY: with no buffer the kernel only says how many bytes the list needs.
    let need = unsafe { libc::proc_pidinfo(pid, LIST_FDS, 0, std::ptr::null_mut::<c_void>(), 0) };
    if need <= 0 {
        return vec![];
    }
    let room = need as usize / each + 32;
    let mut list: Vec<proc_fdinfo> = Vec::with_capacity(room);
    // SAFETY: the vector has room for `room` entries; its length is set to what the kernel wrote.
    unsafe {
        let size = (room * each) as c_int;
        let got = libc::proc_pidinfo(pid, LIST_FDS, 0, list.as_mut_ptr().cast(), size);
        list.set_len((got.max(0) as usize / each).min(room));
    }
    list.iter()
        .filter(|f| f.proc_fdtype == A_SOCKET)
        .map(|f| f.proc_fd)
        .collect()
}

/// A TCP socket's own port and the other end's.
fn ports(pid: pid_t, fd: i32) -> Option<(u16, u16)> {
    let mut info = [0u8; SIZE];
    // SAFETY: the buffer is the size of the struct the kernel writes.
    let got = unsafe {
        libc::proc_pidfdinfo(
            pid,
            fd,
            SOCKET_INFO,
            info.as_mut_ptr().cast(),
            SIZE as c_int,
        )
    };
    let word = |at: usize| i32::from_ne_bytes([info[at], info[at + 1], info[at + 2], info[at + 3]]);
    let port = |at: usize| u16::from_be(word(at) as u16);
    (got as usize == SIZE && word(KIND) == TCP).then(|| (port(OWN_PORT), port(THEIR_PORT)))
}
