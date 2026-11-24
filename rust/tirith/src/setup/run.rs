//! Running another program for the guide: in the owner's sight (a login, an installer, sudo), or quietly for its exit.
use super::ask::Ask;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub type Env = [(String, String)];

/// Runs it on the terminal, with everything it says shown (on the page, in that agent's setup); whether it ended well.
pub fn shown(ask: &Ask, program: &Path, args: &[&str], env: &Env) -> bool {
    if let Some(alias) = ask.on_page() {
        let limit = super::web::capture::limit(alias);
        return super::web::capture::run(alias, program, args, env, limit);
    }
    let mut command = Command::new(program);
    command
        .args(args)
        .envs(env.iter().map(|(k, v)| (k, v)))
        .stdin(ask.stdin());
    match command.status() {
        Ok(done) => done.success(),
        Err(e) => {
            println!("  cannot start {}: {e}", program.display());
            false
        }
    }
}

/// Runs it out of sight; whether it ended well within `limit`, none when it did not end in time or could not start.
pub fn quiet(program: &Path, args: &[&str], env: &Env, limit: Duration) -> Option<bool> {
    let spawned = Command::new(program)
        .args(args)
        .envs(env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let mut child = spawned.ok()?;
    let until = Instant::now() + limit;
    while Instant::now() < until {
        match child.try_wait() {
            Ok(Some(done)) => return Some(done.success()),
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => break,
        }
    }
    let _ = (child.kill(), child.wait());
    None
}

/// What a program prints, trimmed; none when it fails.
pub fn output(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}
