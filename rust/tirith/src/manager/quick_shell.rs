//! A bot's agent is spared the owner's shell profile at every session. An agent's first shell command of a
//! session waits for its own shell to start as a terminal does, profile and all (5 seconds on a machine whose
//! profile loads nvm), and a routine's run is often a session of its own. Where the catalog names the agent's two
//! variables (`shell_var`: the shell its commands run in; `env_file_var`: a file that shell reads before each
//! command), a bot's agent gets the other shell of the two — the one whose own start reads nothing slow — and the
//! owner's environment from the capture of `os/login_env.rs`. Only where the owner's own shell takes a second or
//! more to start: the other shell has none of the profile's functions (nvm's, say), and a quick profile is not worth
//! that. A project's manager and the lanes always keep the owner's shell: what they build may call those functions.
use crate::app::Shared;
use std::path::Path;
#[cfg(unix)]
use std::sync::atomic::{AtomicBool, Ordering};

/// The quick shell, once the capture is taken.
#[cfg(unix)]
static READY: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
#[cfg(unix)]
static BUSY: AtomicBool = AtomicBool::new(false);
/// A login shell that takes this long to start is worth sparing.
#[cfg(unix)]
const SLOW: std::time::Duration = std::time::Duration::from_secs(1);

/// The shell of the two an agent can run its commands in that is not the owner's and starts in under half a second.
#[cfg(unix)]
fn quick() -> Option<String> {
    let owners = std::env::var("SHELL").unwrap_or_default();
    let fast = |shell: &&str| {
        let began = std::time::Instant::now();
        let mut run = std::process::Command::new(shell);
        let ended = run
            .args(["-lc", "exit"])
            .stdin(std::process::Stdio::null())
            .status();
        ended.is_ok_and(|s| s.success()) && began.elapsed().as_millis() < 500
    };
    let others = ["/bin/bash", "/bin/zsh"].into_iter();
    let mut found = others.filter(|s| !owners.ends_with(&s[4..]) && Path::new(s).is_file());
    found.find(fast).map(String::from)
}

/// At the daemon's start, and again when a profile file changed: in the background, the profile being what is slow.
#[cfg(unix)]
pub fn prepare(home: &Path) {
    if BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    let home = home.to_path_buf();
    std::thread::spawn(move || {
        let began = std::time::Instant::now();
        let taken = crate::os::login_run::capture(&home);
        // Only an owner whose shell is slow to start is spared it: a quick one keeps its functions for the bots too.
        let slow = began.elapsed() >= SLOW;
        *READY.lock().unwrap() = (taken && slow).then(quick).flatten();
        BUSY.store(false, Ordering::SeqCst);
    });
}

/// Windows has no login shell to spare anyone.
#[cfg(not(unix))]
pub fn prepare(_home: &Path) {}

/// The two variables for a bot's agent (`conv`: its conversation; `program`: what is started), when the capture
/// is ready and still true to the profile; else none, and its shell reads the profile itself as before.
pub fn env(app: &Shared, conv: &str, program: &str) -> Vec<(String, String)> {
    let agents = app.cfg().factory.commands();
    let alias = agents.iter().find(|(_, command)| command == program);
    let known = alias.and_then(|(alias, _)| crate::setup::known_agent(alias));
    let bot = crate::bots::bot(app, conv).is_some();
    let named = known.filter(|k| bot && !k.shell_var.is_empty() && !k.env_file_var.is_empty());
    let Some(known) = named else {
        return vec![];
    };
    given(&app.home, known.shell_var, known.env_file_var)
}

#[cfg(unix)]
fn given(home: &Path, shell_var: String, file_var: String) -> Vec<(String, String)> {
    if !crate::os::login_env::current(home) {
        *READY.lock().unwrap() = None;
        prepare(home); // a profile file changed: this session reads it itself, the next ones the new capture
        return vec![];
    }
    let file = crate::os::login_env::file(home).display().to_string();
    let shell = READY.lock().unwrap().clone();
    shell.map_or(vec![], |shell| vec![(shell_var, shell), (file_var, file)])
}

#[cfg(not(unix))]
fn given(_home: &Path, _shell_var: String, _file_var: String) -> Vec<(String, String)> {
    vec![]
}
