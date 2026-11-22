//! One program started for a caller: under the one profile, on the caller's own stdin, stdout and stderr.
use super::{limits::profile, Request};
use std::os::fd::OwnedFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

pub(super) fn spawn(r: &Request, ports: &[u16], io: [OwnedFd; 3]) -> Result<Child, String> {
    if !r.program.starts_with('/') || !r.cwd.starts_with('/') {
        return Err("a program and a folder by their whole paths".into()); // a program named like an option would be one
    }
    let (text, params) = profile(r, ports);
    let mut command = Command::new(super::super::profile::SANDBOX_EXEC);
    for (name, value) in params {
        command.arg("-D").arg(format!("{name}={value}"));
    }
    let env = r
        .env
        .iter()
        .filter(|(k, v)| !k.is_empty() && !k.contains(['=', '\0']) && !v.contains('\0'));
    let [stdin, stdout, stderr] = io;
    command
        .args(["-p", &text])
        .arg(&r.program)
        .args(&r.args)
        .env_clear()
        .envs(env.map(|(k, v)| (k, v)))
        .current_dir(&r.cwd)
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .process_group(0)
        .spawn()
        .map_err(|e| format!("cannot start {}: {e}", r.program))
}
