//! Which way this home's daemon is kept running.
use super::{systemd, unit::Unit};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Systemd,
    Launchd,
    /// launchd, at boot, with nobody logged in.
    LaunchdBoot,
    /// Started by hand: nothing brings it back.
    Plain,
}

impl Kind {
    /// The installed one.
    pub fn installed(u: &Unit) -> Kind {
        if cfg!(target_os = "macos") {
            if u.boot_file().exists() {
                return Kind::LaunchdBoot;
            }
            if u.agent_file().exists() {
                return Kind::Launchd;
            }
        } else if u.systemd_file().exists() {
            return Kind::Systemd;
        }
        Kind::Plain
    }

    /// The one to install here.
    pub fn wanted(boot: bool) -> Kind {
        if cfg!(target_os = "macos") {
            return if boot {
                Kind::LaunchdBoot
            } else {
                Kind::Launchd
            };
        }
        if systemd::usable() {
            Kind::Systemd
        } else {
            Kind::Plain
        }
    }

    pub fn said(self) -> &'static str {
        match self {
            Kind::Systemd => "a systemd user service",
            Kind::Launchd => "a launchd service of yours",
            Kind::LaunchdBoot => "a launchd service that starts at boot",
            Kind::Plain => "started by hand (no service manager brings it back)",
        }
    }
}
