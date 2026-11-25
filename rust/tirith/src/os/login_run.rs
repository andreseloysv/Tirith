//! Taking the capture of `login_env.rs`: the owner's login shell run once, the way an agent starts its own (a
//! login shell, not an interactive one). What only the owner's terminal has, the exports of their interactive
//! profile (keys often live there), an agent's shell never had and is not handed here either.
use super::login_env::{file, lines, parse, Vars, MARK};
use crate::os::Mode;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Runs the owner's login shell once and keeps what it exports; whether it could.
/// A profile that waits for a key never ends: after half a minute it is ended and nothing is kept.
pub fn capture(home: &Path) -> bool {
    capture_with(
        home,
        &std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into()),
    )
}

/// The same, with the login shell named.
pub fn capture_with(home: &Path, shell: &str) -> bool {
    let script = format!("printf '%s\\0' {MARK}; exec /usr/bin/env -0");
    let mut run = Command::new(shell);
    run.args(["-lc", &script]).stdin(Stdio::null());
    let Ok(mut child) = run.stdout(Stdio::piped()).stderr(Stdio::null()).spawn() else {
        return false;
    };
    let mut pipe = child.stdout.take();
    let reader = std::thread::spawn(move || {
        let mut all = Vec::new();
        let _ = pipe.as_mut().map(|p| p.read_to_end(&mut all));
        all
    });
    let until = Instant::now() + Duration::from_secs(30);
    let ended = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() < until => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = (child.kill(), child.wait());
                break false;
            }
        }
    };
    let Some(vars) = parse(&reader.join().unwrap_or_default()).filter(|_| ended) else {
        return false; // the capture there, if any, stays
    };
    let base: Vars = std::env::vars_os()
        .map(|(k, v)| (k.to_string_lossy().into(), v.to_string_lossy().into()))
        .collect();
    let mut open = std::fs::OpenOptions::new();
    let kept = open
        .write(true)
        .create(true)
        .truncate(true)
        .with_mode(0o600);
    kept.open(file(home))
        .and_then(|mut out| out.write_all(lines(&vars, &base).as_bytes()))
        .is_ok()
}

#[cfg(test)]
#[path = "../../checks/login_env.rs"]
mod tests;
