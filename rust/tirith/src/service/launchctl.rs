//! launchctl on one job: in the owner's domain, or the system's with sudo.
use super::unit::Unit;
use crate::setup::{find, run, Ask};
use std::path::{Path, PathBuf};
use std::time::Duration;

const LAUNCHCTL: &str = "/bin/launchctl";

pub(super) struct Launchctl<'a> {
    ask: &'a Ask,
    label: String,
    file: PathBuf,
    /// The domains to try, the first first.
    domains: Vec<String>,
    boot: bool,
}

impl<'a> Launchctl<'a> {
    pub fn new(u: &Unit, ask: &'a Ask, boot: bool) -> Launchctl<'a> {
        let uid = unsafe { libc::geteuid() };
        let domains = if boot {
            vec!["system".to_string()]
        } else {
            vec![format!("gui/{uid}"), format!("user/{uid}")] // user: nobody at the screen (SSH)
        };
        Launchctl {
            ask,
            label: u.label(),
            file: if boot { u.boot_file() } else { u.agent_file() },
            domains,
            boot,
        }
    }

    fn run(&self, args: &[&str]) -> bool {
        match find::program("sudo").filter(|_| self.boot) {
            Some(sudo) => run::shown(self.ask, &sudo, &[&[LAUNCHCTL][..], args].concat(), &[]),
            None => run::quiet(Path::new(LAUNCHCTL), args, &[], Duration::from_secs(30)),
        }
    }

    fn at(&self) -> Option<String> {
        self.domains
            .iter()
            .map(|d| format!("{d}/{}", self.label))
            .find(|job| run::output(LAUNCHCTL, &["print", job.as_str()]).is_some())
    }

    pub fn loaded(&self) -> bool {
        self.at().is_some()
    }

    pub fn load(&self) -> Result<(), String> {
        let file = self.file.display().to_string();
        for _ in 0..8 {
            if self
                .domains
                .iter()
                .any(|d| self.run(&["bootstrap", d, &file]))
            {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(500)); // the job it replaces may still be leaving
        }
        Err(format!("launchctl could not load {file}"))
    }

    pub fn unload(&self) -> Result<(), String> {
        let job = self.at().ok_or("the service is not loaded")?;
        self.run(&["bootout", &job])
            .then_some(())
            .ok_or(format!("launchctl could not stop {job}"))
    }

    pub fn kick(&self, again: bool) -> Result<(), String> {
        let job = self.at().ok_or("the service is not loaded")?;
        let args: &[&str] = if again {
            &["kickstart", "-k", &job]
        } else {
            &["kickstart", &job]
        };
        self.run(args)
            .then_some(())
            .ok_or(format!("launchctl could not start {job}"))
    }
}
