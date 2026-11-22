//! launchd: a LaunchAgent in the user's `gui/<uid>` domain. A LaunchDaemon would have no Safari, no Edge and no keychain: they need the user's session.
use super::manager::{Manager, State};
use super::run::{output, run};
use std::path::Path;
use std::time::Duration;

pub(super) const LABEL: &str = "com.tirith.serve";

pub(super) fn manager() -> Manager {
    Manager {
        file: "Library/LaunchAgents/com.tirith.serve.plist",
        text: super::plist::plist,
        start,
        stop,
        forget: || Ok(()),
        state,
        note: None,
    }
}

fn domain() -> String {
    format!("gui/{}", unsafe { libc::getuid() })
}

fn target() -> String {
    format!("{}/{LABEL}", domain())
}

fn start(file: &Path) -> Result<(), String> {
    stop()?;
    run("launchctl", &["enable", &target()])?; // bootstrap refuses a label that was ever disabled
    let file = file.display().to_string();
    run("launchctl", &["bootstrap", &domain(), &file])
        .map(drop)
        .map_err(|e| {
            format!(
                "{e}\n(a LaunchAgent loads only while the user is logged in on the Mac's screen)"
            )
        })
}

/// bootout can return before the job is gone, and a bootstrap then fails: it waits until launchd forgot it.
fn stop() -> Result<(), String> {
    if !state()?.loaded {
        return Ok(());
    }
    let asked = run("launchctl", &["bootout", &target()]);
    for _ in 0..150 {
        if !state()?.loaded {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    asked.and(Err(format!("launchd still has {LABEL} after 30 s")))
}

fn state() -> Result<State, String> {
    let out = output("launchctl", &["print", &target()])?;
    Ok(if out.status.success() {
        read_print(&String::from_utf8_lossy(&out.stdout))
    } else {
        State::default() // print fails for a label that is not loaded
    })
}

pub(super) fn read_print(text: &str) -> State {
    State {
        loaded: true,
        pid: text
            .lines()
            .find_map(|l| l.trim().strip_prefix("pid = ")?.parse().ok()),
    }
}
