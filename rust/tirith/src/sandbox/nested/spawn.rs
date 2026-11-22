//! One process started for a caller under the profile of its part, with that environment alone.
use super::limits::{profile, Around, Part};
use super::Request;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

pub(super) fn spawn(
    r: &Request,
    around: &Around,
    part: Part,
    io: [Stdio; 3],
) -> Result<Child, String> {
    let (text, params) = profile(r, around, part);
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
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr)
        .process_group(0)
        .spawn()
        .map_err(|e| format!("cannot start {}: {e}", r.program))
}
