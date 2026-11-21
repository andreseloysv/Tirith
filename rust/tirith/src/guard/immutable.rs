//! The immutable flag on installed hooks.

/// Sets or clears the immutable flag (macOS `uchg`): installed hooks cannot be changed or deleted, by agents or anyone, until the installer clears it. Elsewhere a no-op.
#[cfg(target_os = "macos")]
pub(super) fn immutable(path: &std::path::Path, on: bool) {
    use std::os::macos::fs::MetadataExt;
    std::fs::metadata(path)
        .ok()
        .zip(std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).ok())
        .into_iter()
        .for_each(|(m, c)| {
            unsafe {
                libc::chflags(
                    c.as_ptr(),
                    if on {
                        m.st_flags() | libc::UF_IMMUTABLE
                    } else {
                        m.st_flags() & !libc::UF_IMMUTABLE
                    },
                )
            };
        });
}
#[cfg(not(target_os = "macos"))]
pub(super) fn immutable(_path: &std::path::Path, _on: bool) {}
