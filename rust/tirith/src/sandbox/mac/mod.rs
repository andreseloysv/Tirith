//! macOS: whether a process runs under NeoBot's own sandbox profile. The profile allows everything but a few things, one a name nothing registers (`OUTSIDE`): denied that and allowed another is NeoBot's; a browser's sandbox denies both.
mod socket;

use libc::{c_char, c_int, pid_t};
use std::ffi::CStr;

/// The name the profile denies (`profile.rs`), and one it does not.
const OUTSIDE: &CStr = c"app.tirith.outside";
const CONTROL: &CStr = c"app.tirith.control";
/// What a tighter profile denies as well (`nested/limits.rs`).
const TIGHTER: &CStr = c"app.tirith.tighter";
/// A global Mach name as the filter, and no line in the system's log.
const NAMED: c_int = 2 | 0x4000_0000;

extern "C" {
    fn sandbox_check(pid: pid_t, operation: *const c_char, kind: c_int, ...) -> c_int;
}

/// 0 when `pid` may look `name` up, 1 when it may not, -1 when there is no such process.
fn check(pid: pid_t, name: &CStr) -> c_int {
    // SAFETY: both strings end in NUL and live through the call, which only reads them.
    unsafe { sandbox_check(pid, c"mach-lookup".as_ptr(), NAMED, name.as_ptr()) }
}

/// Whether `pid` is denied `name` and not the control: a process that is gone counts as if it were.
fn denied(pid: pid_t, name: &CStr) -> bool {
    match (check(pid, name), check(pid, CONTROL)) {
        (-1, _) | (_, -1) => true,
        (inner, control) => inner != 0 && control == 0,
    }
}

/// Whether `pid` runs under NeoBot's profile.
pub(super) fn ours(pid: pid_t) -> bool {
    denied(pid, OUTSIDE)
}

/// Whether `pid` runs under a profile tighter than NeoBot's.
pub(super) fn tighter(pid: pid_t) -> bool {
    denied(pid, TIGHTER)
}

/// Whether the connection from port `from` to port `to` of this Mac is a confined process's. One nobody of this user holds is never the owner's.
pub(super) fn confined(from: u16, to: u16) -> bool {
    // SAFETY: getpid only reads this process's own id.
    if ours(unsafe { libc::getpid() }) {
        return false; // the daemon runs in such a sandbox itself (a lane's test): nothing tells them apart
    }
    let holders = socket::holders(from, to);
    holders.is_empty() || holders.into_iter().any(ours)
}

/// Whether process `pid` holds the end at port `from` of the connection to port `to`.
pub(super) fn held_by(from: u16, to: u16, pid: pid_t) -> bool {
    socket::holders(from, to).contains(&pid)
}
