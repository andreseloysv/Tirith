//! Server mode: the daemon as a service of this machine (systemd or launchd), or started by hand where there is neither.
mod alive;
mod cli;
mod kind;
mod launchctl;
mod launchd;
mod plain;
mod plist;
mod systemd;
mod systemd_text;
mod unit;

pub use cli::cli;
pub use kind::Kind;
pub use unit::Unit;

use crate::setup::{daemon, Ask};

#[cfg(test)]
#[path = "../../checks/service.rs"]
mod tests;

/// Installs the service and waits until its daemon answers: how it runs, and what is left for the owner.
pub fn install(u: &Unit, ask: &Ask, boot: bool) -> Result<(Kind, Vec<String>), String> {
    let (had, kind) = (Kind::installed(u), Kind::wanted(boot));
    if daemon::answering().is_some() && had != kind {
        stop(u, ask)?; // started by hand, or the other kind of service
    }
    if matches!(
        (had, kind),
        (Kind::Launchd, Kind::LaunchdBoot) | (Kind::LaunchdBoot, Kind::Launchd)
    ) {
        launchd::uninstall(u, ask, had == Kind::LaunchdBoot)?;
    }
    let mut notes = match kind {
        Kind::Systemd => systemd::install(u, ask)?,
        Kind::Launchd => launchd::install(u, ask, false)?,
        Kind::LaunchdBoot => launchd::install(u, ask, true)?,
        Kind::Plain => {
            plain::stop(u)?;
            plain::start(u)?;
            vec!["this system has no systemd user session, so nothing starts OpenBot again after a reboot: run `tirith service start` then".to_string()]
        }
    };
    daemon::up(&u.home)?;
    notes.retain(|n| !n.is_empty());
    Ok((kind, notes))
}

pub fn start(u: &Unit, ask: &Ask) -> Result<Kind, String> {
    let kind = Kind::installed(u);
    if !daemon::answering().is_some_and(|v| daemon::is_of(&v, &u.home)) {
        if daemon::answering().is_some() {
            return Err("another OpenBot answers on this port: choose another with tirith setup --port <port>".into());
        }
        match kind {
            Kind::Systemd => systemd::act(u, "start")?,
            Kind::Launchd | Kind::LaunchdBoot => {
                launchd::act(u, ask, kind == Kind::LaunchdBoot, "start")?
            }
            Kind::Plain => plain::start(u)?,
        }
    }
    daemon::up(&u.home).map(|_| kind)
}

pub fn stop(u: &Unit, ask: &Ask) -> Result<(), String> {
    let kind = Kind::installed(u);
    match kind {
        Kind::Systemd => systemd::act(u, "stop")?,
        Kind::Launchd | Kind::LaunchdBoot => {
            launchd::act(u, ask, kind == Kind::LaunchdBoot, "stop")?
        }
        Kind::Plain => {}
    }
    plain::stop(u)
}

pub fn uninstall(u: &Unit, ask: &Ask) -> Result<(), String> {
    let kind = Kind::installed(u);
    match kind {
        Kind::Systemd => systemd::uninstall(u)?,
        Kind::Launchd | Kind::LaunchdBoot => launchd::uninstall(u, ask, kind == Kind::LaunchdBoot)?,
        Kind::Plain => {}
    }
    plain::stop(u)
}
