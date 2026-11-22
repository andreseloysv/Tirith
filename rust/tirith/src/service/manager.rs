//! The service manager of this OS, as the commands use it.
use super::spec::Spec;
use super::{launchd, systemd};
use std::path::{Path, PathBuf};

/// What the service manager knows of the service.
#[derive(Debug, Default, PartialEq)]
pub(super) struct State {
    pub loaded: bool,
    pub pid: Option<u32>,
}

pub(super) struct Manager {
    /// Where the service's file lives under the user's home.
    pub file: &'static str,
    pub text: fn(&Spec) -> String,
    /// Loads the file just written and starts it, replacing the one that runs.
    pub start: fn(&Path) -> Result<(), String>,
    pub stop: fn() -> Result<(), String>,
    /// After the file is removed.
    pub forget: fn() -> Result<(), String>,
    pub state: fn() -> Result<State, String>,
    /// What the owner still has to know after installing.
    pub note: Option<&'static str>,
}

impl Manager {
    pub fn here() -> Result<Manager, String> {
        match std::env::consts::OS {
            "macos" => Ok(launchd::manager()),
            "linux" => Ok(systemd::manager()),
            os => Err(format!(
                "tirith service works on macOS and Linux, not on {os}"
            )),
        }
    }

    pub fn path(&self) -> PathBuf {
        crate::config::home_dir().join(self.file)
    }
}
