//! The full name of the person this account belongs to, as the computer knows it (for a greeting): the account's
//! display name on Windows, its full name on macOS, the passwd entry's on Linux. None when the computer has none.

#[cfg(windows)]
pub fn full_name() -> Option<String> {
    use windows_sys::Win32::Security::Authentication::Identity::{GetUserNameExW, NameDisplay};
    let mut buf = [0u16; 256];
    let mut size = buf.len() as u32;
    // SAFETY: the buffer holds `size` wide characters; the call writes no more and says how many.
    let ok = unsafe { GetUserNameExW(NameDisplay, buf.as_mut_ptr(), &mut size) };
    let name = String::from_utf16_lossy(&buf[..(size as usize).min(buf.len())]);
    named(ok.then_some(name))
}

#[cfg(target_os = "macos")]
pub fn full_name() -> Option<String> {
    let out = std::process::Command::new("/usr/bin/id")
        .arg("-F")
        .output()
        .ok()?;
    named(
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).into_owned()),
    )
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn full_name() -> Option<String> {
    let user = std::env::var("USER").ok()?;
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;
    let line = passwd
        .lines()
        .find(|l| l.split(':').next() == Some(user.as_str()))?;
    named(
        line.split(':')
            .nth(4)
            .map(|g| g.split(',').next().unwrap_or("").to_string()),
    )
}

fn named(name: Option<String>) -> Option<String> {
    name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty())
}
