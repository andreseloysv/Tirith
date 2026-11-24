//! A variable from the environment this process's own ancestors started with.

/// How far up it looks: a hook is a few processes below its agent.
const FAR: usize = 8;

/// `name` as the nearest ancestor that has it was given it.
pub(super) fn var(name: &str) -> Option<String> {
    let mut pid = crate::os::parent_id()?;
    for _ in 0..FAR {
        if pid <= 1 {
            break;
        }
        if let Some(value) = given(pid).and_then(|env| value_in(&env, name)) {
            return Some(value);
        }
        pid = parent(pid)?;
    }
    None
}

/// The value of `name` in NUL-ended `NAME=value` words.
fn value_in(words: &[u8], name: &str) -> Option<String> {
    let head = format!("{name}=");
    let value = words
        .split(|b| *b == 0)
        .find_map(|word| word.strip_prefix(head.as_bytes()))?;
    String::from_utf8(value.to_vec())
        .ok()
        .filter(|v| !v.is_empty())
}

#[cfg(target_os = "linux")]
fn given(pid: u32) -> Option<Vec<u8>> {
    std::fs::read(format!("/proc/{pid}/environ")).ok()
}

#[cfg(target_os = "linux")]
fn parent(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // the name before it may hold blanks and brackets
    stat.rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

/// Its arguments, then its environment (`KERN_PROCARGS2`), after a count.
#[cfg(target_os = "macos")]
fn given(pid: u32) -> Option<Vec<u8>> {
    let mut name = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid as libc::c_int];
    let mut buf = vec![0u8; 1 << 20];
    let mut size = buf.len();
    // SAFETY: the buffer holds `size` bytes; the kernel writes no more and says how many.
    let got = unsafe {
        libc::sysctl(
            name.as_mut_ptr(),
            name.len() as libc::c_uint,
            buf.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    (got == 0 && size > 4).then(|| buf[4..size.min(buf.len())].to_vec())
}

#[cfg(target_os = "macos")]
fn parent(pid: u32) -> Option<u32> {
    let size = std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int;
    // SAFETY: the struct is the size the kernel is told, and all zero is a valid one.
    unsafe {
        let mut info: libc::proc_bsdinfo = std::mem::zeroed();
        let (pid, at) = (pid as libc::c_int, (&raw mut info).cast());
        let got = libc::proc_pidinfo(pid, libc::PROC_PIDTBSDINFO, 0, at, size);
        (got == size).then_some(info.pbi_ppid)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn given(_pid: u32) -> Option<Vec<u8>> {
    None
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn parent(_pid: u32) -> Option<u32> {
    None
}
