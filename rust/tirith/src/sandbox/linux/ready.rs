//! Whether this Linux lets OpenBot confine what it starts, asked by trying it, and what to do when it does not.
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A no is asked again after this long (the owner may have allowed it meanwhile); a yes holds.
const AGAIN: Duration = Duration::from_secs(15);
static LAST: Mutex<Option<(Instant, Result<(), String>)>> = Mutex::new(None);
const APPARMOR: &str = "/proc/sys/kernel/apparmor_restrict_unprivileged_userns";

pub(in crate::sandbox) fn ready() -> Result<(), String> {
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    match last.as_ref() {
        Some((_, Ok(()))) => return Ok(()),
        Some((at, no)) if at.elapsed() < AGAIN => return no.clone(),
        _ => {}
    }
    let now = tried().map_err(|why| explained(&why));
    *last = Some((Instant::now(), now.clone()));
    now
}

/// The test binary is no `tirith`: to the tests this system confines, as macOS does, and what they start runs as it is.
#[cfg(test)]
fn tried() -> Result<(), String> {
    Ok(())
}

#[cfg(not(test))]
fn tried() -> Result<(), String> {
    let out = std::process::Command::new(super::SELF)
        .args(["net", "run", "--hide", "/dev/zero", "--"]) // both namespaces, tried
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
    out.status.success().then_some(()).ok_or(why)
}

fn explained(why: &str) -> String {
    let restricted = std::fs::read_to_string(APPARMOR).is_ok_and(|t| t.trim() == "1");
    let fix = if restricted {
        let exe = crate::app::exe().unwrap_or_else(|| "tirith".into());
        format!("this Linux allows user namespaces only to programs with a profile: run `sudo {} net allow`", exe.display())
    } else {
        "it needs unprivileged user namespaces (kernel.unprivileged_userns_clone = 1; in Docker, a seccomp profile that allows unshare)".into()
    };
    format!("no namespaces to confine in ({why}); {fix}")
}
