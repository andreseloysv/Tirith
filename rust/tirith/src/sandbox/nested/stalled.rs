//! Whether what runs a limited Codex's commands was stopped: a command can signal its own sandbox, that process included, and a stopped one answers no more.
/// True while process `pid` is stopped.
pub(super) fn stalled(pid: u32) -> bool {
    // SAFETY: an all-zero proc_bsdinfo is a valid one to be written over.
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int;
    // SAFETY: the buffer is the size given, and the kernel writes no more.
    let got = unsafe {
        libc::proc_pidinfo(
            pid as libc::c_int,
            libc::PROC_PIDTBSDINFO,
            0,
            (&raw mut info).cast(),
            size,
        )
    };
    got == size && info.pbi_status == libc::SSTOP
}
