//! The machine: who runs this, its name, its free disk and its load.
use std::path::Path;

/// The effective user id; Windows has none, so one no user has (never root's 0).
#[cfg(unix)]
pub fn uid() -> u32 {
    unsafe { libc::geteuid() }
}

#[cfg(not(unix))]
pub fn uid() -> u32 {
    u32::MAX
}

/// Whether this runs as root (never on Windows, where an administrator changes nothing here).
pub fn is_root() -> bool {
    uid() == 0
}

/// This machine's name.
#[cfg(unix)]
pub fn hostname() -> String {
    let mut buf = [0u8; 256];
    unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
    String::from_utf8_lossy(&buf[..buf.iter().position(|b| *b == 0).unwrap_or(0)]).into_owned()
}

#[cfg(not(unix))]
pub fn hostname() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_default()
}

/// Gigabytes free to this user on the disk that holds `dir`.
#[cfg(unix)]
pub fn free_gb(dir: &Path) -> Option<f64> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(dir.as_os_str().as_bytes()).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    (unsafe { libc::statvfs(c.as_ptr(), &mut s) } == 0)
        .then(|| s.f_bavail as f64 * s.f_frsize as f64 / 1e9)
}

#[cfg(windows)]
pub fn free_gb(dir: &Path) -> Option<f64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain([0]).collect();
    let mut free = 0u64;
    let (none, other) = (std::ptr::null_mut(), std::ptr::null_mut());
    // SAFETY: a NUL-ended name, and the one count asked for written into a local.
    let read = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, none, other) };
    (read != 0).then(|| free as f64 / 1e9)
}

/// The one-minute load average per core; Windows keeps none, so 0.
#[cfg(unix)]
pub fn load() -> f64 {
    let mut avg = [0f64; 3];
    unsafe { libc::getloadavg(avg.as_mut_ptr(), 3) }; // leaves zeros when it fails
    avg[0] / std::thread::available_parallelism().map_or(1, |c| c.get()) as f64
}

#[cfg(not(unix))]
pub fn load() -> f64 {
    0.0
}
