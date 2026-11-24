//! Whether what answers is the service's own process: while another daemon has the port the service retries, with a pid of its own.
use super::{launchctl::Launchctl, systemd, Kind, Unit};
use crate::setup::Ask;
use serde_json::Value;

/// The `alive:` line's words, from the process the service runs and the one that answers.
pub(super) fn said(service: Option<u32>, answer: Option<&Value>) -> String {
    match (service, answer) {
        (_, None) => "no".into(),
        (Some(pid), Some(v)) if v["pid"] == pid => format!("yes (pid {pid})"),
        (Some(_), Some(_)) => "no (a neobot the service did not start has the port: the service takes over when that one stops)".into(),
        (None, Some(_)) => "no (a neobot runs, but not as the service)".into(),
    }
}

/// The process the service runs now.
pub(super) fn pid(u: &Unit, ask: &Ask, kind: Kind) -> Option<u32> {
    match kind {
        Kind::Systemd => systemd::pid(u),
        Kind::Launchd | Kind::LaunchdBoot => {
            Launchctl::new(u, ask, kind == Kind::LaunchdBoot).pid()
        }
        Kind::Plain => None,
    }
}
