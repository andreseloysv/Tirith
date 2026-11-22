//! Linux: the files that prove who calls read as nothing inside (`app/keys.rs`): /dev/null is bound over each in a mount namespace of the confined process's own.
use std::ffi::CString;
use std::io::Error;

/// Hides each of `files` that is there.
pub(super) fn hide(files: &[String]) -> Result<(), String> {
    mount(None, "/", libc::MS_REC | libc::MS_PRIVATE)?; // nothing of it reaches the machine's mounts
    for file in files.iter().filter(|f| std::path::Path::new(f).exists()) {
        mount(Some("/dev/null"), file, libc::MS_BIND)?;
    }
    Ok(())
}

fn mount(from: Option<&str>, to: &str, flags: libc::c_ulong) -> Result<(), String> {
    let text = |path: &str| CString::new(path).map_err(|e| format!("{path}: {e}"));
    let (from, at) = (from.map(text).transpose()?, text(to)?);
    let from = from.as_ref().map_or(std::ptr::null(), |p| p.as_ptr());
    let none = std::ptr::null();
    // SAFETY: the strings end in NUL and live through the call; the kernel takes null for the rest.
    let done = unsafe { libc::mount(from, at.as_ptr(), none, flags, none.cast()) };
    (done == 0)
        .then_some(())
        .ok_or_else(|| format!("hiding {to}: {}", Error::last_os_error()))
}
