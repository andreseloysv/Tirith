//! The service's user works in root's terminal while root runs the guide. Nothing it starts there may type into that terminal (TIOCSTI, TIOCLINUX): what it typed, root's shell would run. A seccomp filter says so and every child inherits it.
use libc::{sock_filter, sock_fprog};

const LOAD: u16 = (libc::BPF_LD | libc::BPF_W | libc::BPF_ABS) as u16;
const EQ: u16 = (libc::BPF_JMP | libc::BPF_JEQ | libc::BPF_K) as u16;
const SAY: u16 = (libc::BPF_RET | libc::BPF_K) as u16;
/// Where `seccomp_data` holds the syscall's number, its architecture and the low half of its second argument.
const NR: u32 = 0;
const ARCH: u32 = 4;
const REQUEST: u32 = 24;

fn op(code: u16, k: u32, jt: u8, jf: u8) -> sock_filter {
    sock_filter { code, jt, jf, k }
}

/// An architecture's id, and its number of ioctl (x86_64 has x32's beside its own).
#[cfg(target_arch = "x86_64")]
const CALLS: [(u32, u32); 3] = [
    (0xC000_003E, 16),
    (0xC000_003E, 0x4000_0202),
    (0x4000_0003, 54), // a 32-bit program's
];
#[cfg(not(target_arch = "x86_64"))]
const CALLS: [(u32, u32); 2] = [(0xC000_00B7, 29), (0x4000_0028, 54)];
const STI: u32 = 0x5412;
const VT: u32 = 0x541C;

fn program() -> Vec<sock_filter> {
    let mut all = vec![];
    for (arch, ioctl) in CALLS {
        all.extend([
            op(LOAD, ARCH, 0, 0),
            op(EQ, arch, 0, 6),
            op(LOAD, NR, 0, 0),
            op(EQ, ioctl, 0, 4),
            op(LOAD, REQUEST, 0, 0),
            op(EQ, STI, 1, 0),
            op(EQ, VT, 0, 1),
            op(SAY, libc::SECCOMP_RET_ERRNO | libc::EPERM as u32, 0, 0),
        ]);
    }
    all.push(op(SAY, libc::SECCOMP_RET_ALLOW, 0, 0));
    all
}

/// From now on this thread, and all it starts, cannot type into a terminal; false where the kernel takes no filter.
pub(super) fn keep() -> bool {
    let mut code = program();
    let filter = sock_fprog {
        len: code.len() as u16,
        filter: code.as_mut_ptr(),
    };
    let (one, none): (libc::c_ulong, libc::c_ulong) = (1, 0);
    let mode = libc::SECCOMP_MODE_FILTER as libc::c_ulong;
    // SAFETY: the filter lives through the call, which copies it; no setuid program is needed after it.
    unsafe {
        libc::prctl(libc::PR_SET_NO_NEW_PRIVS, one, none, none, none) == 0
            && libc::prctl(libc::PR_SET_SECCOMP, mode, &filter as *const sock_fprog) == 0
    }
}
