//! How this system confines what NeoBot starts: `sandbox-exec` on macOS, an empty network namespace on Linux.
use crate::app::Shared;

/// Whether this system can confine, else why not (and what to do).
#[cfg(target_os = "macos")]
pub(super) fn ready() -> Result<(), String> {
    std::path::Path::new(super::profile::SANDBOX_EXEC)
        .exists()
        .then_some(())
        .ok_or_else(|| "macOS's sandbox-exec is missing".to_string())
}

#[cfg(target_os = "linux")]
pub(super) use super::linux::ready;

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(super) fn ready() -> Result<(), String> {
    Err("the sandbox needs macOS or Linux".into())
}

/// `program args…` as it starts confined.
#[cfg(target_os = "linux")]
pub(super) fn argv(app: &Shared, program: String, args: Vec<String>) -> (String, Vec<String>) {
    let cfg = app.cfg();
    let mut ports = vec![cfg.port, super::profile::proxy_port(app)];
    let more: Vec<u16> = crate::config::get(&cfg.raw, &["network", "loopback"]).unwrap_or_default();
    for port in more {
        if !ports.contains(&port) {
            ports.push(port);
        }
    }
    let hidden: Vec<String> = crate::app::kept(&app.home)
        .iter()
        .map(|f| f.display().to_string())
        .collect();
    super::linux::argv(&ports, &hidden, program, args)
}

#[cfg(not(target_os = "linux"))]
pub(super) fn argv(app: &Shared, program: String, args: Vec<String>) -> (String, Vec<String>) {
    let hidden = super::hidden::rules(&crate::app::kept(&app.home))
        + &super::hidden::folder(&app.home.join("auth"));
    let args = [
        "-p".to_string(),
        format!("{}{hidden}", super::profile::PROFILE),
        program,
    ]
    .into_iter()
    .chain(args)
    .collect();
    (super::profile::SANDBOX_EXEC.into(), args)
}

/// `neobot net run …`, the launcher, answered before anything else of the command line runs (it must still have one thread): its exit code.
pub fn launcher(args: &[String]) -> Option<i32> {
    let [net, run, rest @ ..] = args else {
        return None;
    };
    (net == "net" && run == "run").then(|| {
        #[cfg(target_os = "linux")]
        return super::linux::launcher(rest);
        #[cfg(not(target_os = "linux"))]
        {
            let _ = rest;
            eprintln!("neobot net run is Linux's sandbox launcher");
            2
        }
    })
}

/// `neobot net allow`.
pub(super) fn allow() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return super::linux::allow();
    #[cfg(not(target_os = "linux"))]
    Err("neobot net allow is for Linux: macOS needs nothing".into())
}
