//! systemd: a user unit, in the user's own session like the LaunchAgent.
use super::manager::{Manager, State};
use super::run::run;
use std::path::Path;

const UNIT: &str = "tirith.service";

pub(super) fn manager() -> Manager {
    Manager {
        file: ".config/systemd/user/tirith.service",
        text: super::unit::unit,
        start,
        stop,
        forget,
        state,
        note: Some(
            "systemd stops it at logout and starts it only at login: run `loginctl enable-linger` to keep it running without a session",
        ),
    }
}

fn systemctl(args: &[&str]) -> Result<String, String> {
    run("systemctl", &[&["--user"], args].concat())
}

fn start(_file: &Path) -> Result<(), String> {
    systemctl(&["daemon-reload"])?;
    systemctl(&["enable", UNIT])?;
    systemctl(&["restart", UNIT]).map(drop)
}

fn stop() -> Result<(), String> {
    systemctl(&["disable", "--now", UNIT]).map(drop)
}

fn forget() -> Result<(), String> {
    systemctl(&["daemon-reload"]).map(drop)
}

fn state() -> Result<State, String> {
    let props = ["show", UNIT, "-p", "LoadState", "-p", "MainPID"];
    systemctl(&props).map(|text| read_show(&text))
}

pub(super) fn read_show(text: &str) -> State {
    let value = |key: &str| text.lines().find_map(|l| l.strip_prefix(key));
    State {
        loaded: value("LoadState=") == Some("loaded"),
        pid: value("MainPID=")
            .and_then(|pid| pid.parse().ok())
            .filter(|pid| *pid != 0),
    }
}
