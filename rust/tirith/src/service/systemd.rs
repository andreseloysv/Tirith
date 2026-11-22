//! On Linux: a systemd user service that starts at boot (linger) and needs no root.
use super::{systemd_text, unit::Unit};
use crate::setup::{find, run, Ask};
use std::process::{Command, Stdio};

/// `systemctl --user`, also from a `su` shell that lost XDG_RUNTIME_DIR.
fn systemctl(args: &[&str]) -> Result<(), String> {
    let mut command = Command::new("systemctl");
    command.arg("--user").args(args).stdin(Stdio::null());
    let runtime = format!("/run/user/{}", unsafe { libc::geteuid() });
    if std::env::var_os("XDG_RUNTIME_DIR").is_none() && std::path::Path::new(&runtime).is_dir() {
        command.env("XDG_RUNTIME_DIR", runtime);
    }
    let out = command.output().map_err(|e| format!("systemctl: {e}"))?;
    let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
    out.status
        .success()
        .then_some(())
        .ok_or_else(|| format!("systemctl --user {}: {why}", args.join(" ")))
}

pub(super) fn usable() -> bool {
    cfg!(target_os = "linux") && systemctl(&["show-environment"]).is_ok()
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
    let uid = unsafe { libc::geteuid() }.to_string();
    let query = ["show-user", uid.as_str(), "--property=Linger", "--value"];
    let on = || run::output("loginctl", &query).as_deref() == Some("yes");
    if on() || (run::output("loginctl", &["enable-linger"]).is_some() && on()) {
        return None;
    }
    let user = run::output("id", &["-un"]).unwrap_or(uid.clone());
    if let Some(sudo) = find::program("sudo").filter(|_| ask.person()) {
        println!(
            "  So that Tirith starts at boot, with nobody logged in, sudo asks for your password:"
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
    systemctl(&[what, &u.name()])
}

pub(super) fn uninstall(u: &Unit) -> Result<(), String> {
    systemctl(&["disable", "--now", &u.name()])?;
    let file = u.systemd_file();
    std::fs::remove_file(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    systemctl(&["daemon-reload"])
}
