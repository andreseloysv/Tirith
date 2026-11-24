//! On Linux: a systemd user service that starts at boot (linger) and needs no root.
use super::{systemd_text, unit::Unit};
use crate::setup::{root, run, Ask};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

/// `systemctl --user`, also from a `su` shell that lost XDG_RUNTIME_DIR.
fn systemctl(args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("systemctl");
    command.arg("--user").args(args).stdin(Stdio::null());
    let runtime = format!("/run/user/{}", crate::os::uid());
    if std::env::var_os("XDG_RUNTIME_DIR").is_none() && std::path::Path::new(&runtime).is_dir() {
        command.env("XDG_RUNTIME_DIR", runtime);
    }
    let out = command.output().map_err(|e| format!("systemctl: {e}"))?;
    let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .ok_or_else(|| format!("systemctl --user {}: {why}", args.join(" ")))
}

/// The process the unit runs now.
pub(super) fn pid(u: &Unit) -> Option<u32> {
    let shown = systemctl(&["show", "-p", "MainPID", "--value", &u.name()]).ok()?;
    shown.parse().ok().filter(|pid| *pid != 0)
}

/// Whether this user's systemd answers. Linger makes one for a user nobody signed in as (a server's), and it takes a moment.
pub(super) fn usable() -> bool {
    if !cfg!(target_os = "linux") || !Path::new("/run/systemd/system").is_dir() {
        return false;
    }
    let ready = || systemctl(&["show-environment"]).is_ok();
    if ready() {
        return true;
    }
    let uid = crate::os::uid().to_string();
    let query = ["show-user", uid.as_str(), "--property=Linger", "--value"];
    let lingers = run::output("loginctl", &["enable-linger"]).is_some()
        || run::output("loginctl", &query).as_deref() == Some("yes");
    lingers
        && (0..30).any(|_| {
            std::thread::sleep(Duration::from_millis(500));
            ready()
        })
}

pub(super) fn install(u: &Unit, ask: &Ask) -> Result<Vec<String>, String> {
    let (file, text) = (u.systemd_file(), systemd_text::text(u, &u.log()?)?);
    let failed = |e: std::io::Error| format!("{}: {e}", file.display());
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(failed)?;
    }
    std::fs::write(&file, text).map_err(failed)?;
    systemctl(&["daemon-reload"])?;
    systemctl(&["enable", &u.name()])?;
    systemctl(&["restart", &u.name()])?;
    Ok(linger(ask).into_iter().collect())
}

/// Linger keeps the user's services up from boot and after logout: none when it is on, else what is left to do.
fn linger(ask: &Ask) -> Option<String> {
    let uid = crate::os::uid().to_string();
    let query = ["show-user", uid.as_str(), "--property=Linger", "--value"];
    let on = || run::output("loginctl", &query).as_deref() == Some("yes");
    if on() || (run::output("loginctl", &["enable-linger"]).is_some() && on()) {
        return None;
    }
    let user = run::output("id", &["-un"]).unwrap_or(uid.clone());
    if let Some(sudo) = root::sudo().filter(|_| ask.person()) {
        println!(
            "  So that NeoBot starts at boot, with nobody logged in, sudo asks for your password:"
        );
        let args = ["loginctl", "enable-linger", user.as_str()];
        if run::shown(ask, &sudo, &args, &[]) && on() {
            return None;
        }
    }
    Some(format!(
        "to start at boot with nobody logged in, once: sudo loginctl enable-linger {user}"
    ))
}

pub(super) fn act(u: &Unit, what: &str) -> Result<(), String> {
    systemctl(&[what, &u.name()]).map(drop)
}

pub(super) fn uninstall(u: &Unit) -> Result<(), String> {
    systemctl(&["disable", "--now", &u.name()])?;
    let file = u.systemd_file();
    std::fs::remove_file(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    systemctl(&["daemon-reload"]).map(drop)
}
