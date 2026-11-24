//! A program the page's setup runs: out of sight (no window of its own on Windows), what it prints read into the agent's job, its input held open for a code pasted on the page.
use super::job;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// What the job hears when its program was stopped for taking too long.
pub const STOPPED: &str = "\nit took too long and was stopped\n";

/// How long a program may run: a sign-in waits an hour for the owner (who may come back to it later); anything
/// else, a slow installer included, 20 minutes.
pub fn limit(alias: &str) -> Duration {
    let signs_in = job::get(alias).is_some_and(|j| j.step == "signin");
    Duration::from_secs(if signs_in { 60 * 60 } else { 20 * 60 })
}

/// Whether it ended well within `limit`.
pub fn run(
    alias: &str,
    program: &Path,
    args: &[&str],
    env: &[(String, String)],
    limit: Duration,
) -> bool {
    let mut command = Command::new(program);
    command
        .args(args)
        .envs(env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hidden(&mut command);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            job::heard(alias, &format!("cannot start {}: {e}\n", program.display()));
            return false;
        }
    };
    if let Some(input) = child.stdin.take() {
        super::input::hold(alias, input);
    }
    for out in [
        child
            .stdout
            .take()
            .map(|o| Box::new(o) as Box<dyn Read + Send>),
        child
            .stderr
            .take()
            .map(|e| Box::new(e) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .flatten()
    {
        // never joined: what the program leaves running may hold its output open
        let alias = alias.to_string();
        std::thread::spawn(move || read(&alias, out));
    }
    let until = Instant::now() + limit;
    let ended = loop {
        match child.try_wait() {
            Ok(Some(done)) => break done.success(),
            Ok(None) if Instant::now() < until => std::thread::sleep(Duration::from_millis(200)),
            _ => {
                let _ = (child.kill(), child.wait());
                job::heard(alias, STOPPED);
                break false;
            }
        }
    };
    std::thread::sleep(Duration::from_millis(300)); // its last words
    ended
}

fn read(alias: &str, mut out: Box<dyn Read + Send>) {
    let mut buf = [0u8; 4096];
    while let Ok(n) = out.read(&mut buf) {
        if n == 0 {
            break;
        }
        job::heard(alias, &String::from_utf8_lossy(&buf[..n]));
    }
}

/// No console window: the daemon's own console is hidden, and a program started with a window of its own would show one.
#[cfg(windows)]
fn hidden(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hidden(_command: &mut Command) {}
