//! Running the service manager's command.
use std::process::{Command, Output};

pub(super) fn output(program: &str, args: &[&str]) -> Result<Output, String> {
    Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("cannot run {program}: {e}"))
}

/// What it printed, or the command that failed with what it said.
pub(super) fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let out = output(program, args)?;
    let said = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim().to_string();
    out.status
        .success()
        .then(|| said(&out.stdout))
        .ok_or_else(|| {
            format!(
                "{program} {} failed ({}): {}",
                args.join(" "),
                out.status,
                said(&out.stderr)
            )
        })
}
