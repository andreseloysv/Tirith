//! Handing descriptors to another process over a Unix socket (SCM_RIGHTS): Linux's launcher gets its relay's sockets, macOS's broker its caller's stdio.
use std::io::Error;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::net::UnixStream;

/// The most sockets handed over at once (one per forwarded port).
#[cfg(target_os = "linux")]
pub(in crate::sandbox) const MOST: usize = 32;

#[cfg(target_os = "linux")]
const CLOEXEC: libc::c_int = libc::MSG_CMSG_CLOEXEC;
#[cfg(not(target_os = "linux"))]
const CLOEXEC: libc::c_int = 0;

/// Room for the control message, aligned as the kernel wants it.
type Control = [u64; 32];

fn message(byte: &mut libc::iovec, control: &mut Control, space: usize) -> libc::msghdr {
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = byte;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr().cast();
    msg.msg_controllen = space as _;
    msg
}

pub(in crate::sandbox) fn send(sock: &UnixStream, fds: &[RawFd]) -> Result<(), String> {
    let (mut byte, mut control) = ([1u8], [0u64; 32]);
    let mut iov = libc::iovec {
        iov_base: byte.as_mut_ptr().cast(),
        iov_len: 1,
    };
    let bytes = std::mem::size_of_val(fds) as u32;
    unsafe {
        let space = libc::CMSG_SPACE(bytes) as usize;
        let mut msg = message(&mut iov, &mut control, space);
        if fds.is_empty() {
            msg.msg_controllen = 0;
        } else {
            let head = libc::CMSG_FIRSTHDR(&msg);
            (*head).cmsg_level = libc::SOL_SOCKET;
            (*head).cmsg_type = libc::SCM_RIGHTS;
            (*head).cmsg_len = libc::CMSG_LEN(bytes) as _;
            let data = libc::CMSG_DATA(head);
            std::ptr::copy_nonoverlapping(fds.as_ptr().cast::<u8>(), data, bytes as usize);
        }
        (libc::sendmsg(sock.as_raw_fd(), &msg, 0) == 1)
            .then_some(())
            .ok_or_else(|| format!("handing the ports over: {}", Error::last_os_error()))
    }
}

/// The sockets the child handed over; none when it ended without doing so.
pub(in crate::sandbox) fn receive(sock: &UnixStream) -> Option<Vec<RawFd>> {
    let (mut byte, mut control) = ([0u8], [0u64; 32]);
    let mut iov = libc::iovec {
        iov_base: byte.as_mut_ptr().cast(),
        iov_len: 1,
    };
    let space = std::mem::size_of::<Control>();
    let mut msg = message(&mut iov, &mut control, space);
    unsafe {
        let got = libc::recvmsg(sock.as_raw_fd(), &mut msg, CLOEXEC);
        (got == 1).then_some(())?;
        let head = libc::CMSG_FIRSTHDR(&msg);
        if head.is_null() || (*head).cmsg_type != libc::SCM_RIGHTS {
            return Some(vec![]);
        }
        let bytes = (*head).cmsg_len as usize - libc::CMSG_LEN(0) as usize;
        let data = libc::CMSG_DATA(head).cast::<RawFd>();
        let count = bytes / std::mem::size_of::<RawFd>();
        let fds: Vec<RawFd> = (0..count).map(|i| data.add(i).read_unaligned()).collect();
        #[cfg(target_os = "macos")]
        for fd in &fds {
            libc::fcntl(*fd, libc::F_SETFD, libc::FD_CLOEXEC); // no other child inherits a caller's
        }
        Some(fds)
    }
}
