//! `codex` inside Tirith's sandbox (the wrapper of `<home>/net/bin`): its limit is read and the daemon is asked to apply it.
use super::{ask::ask, asked, program::whole, settings, WORD};
use crate::sandbox::mac;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;

/// `tirith nested-codex <codex> args…`, answered before the command line is read.
pub fn launcher(args: &[String]) -> Option<i32> {
    let [word, program, rest @ ..] = args else {
        return None;
    };
    (word == WORD).then(|| run(program, rest))
}

/// `argv` of a Codex Tirith starts inside its sandbox, through this client.
pub fn inside(argv: &mut Vec<String>) {
    if let Some(exe) = crate::app::exe() {
        argv.splice(0..0, [exe.display().to_string(), WORD.to_string()]);
    }
}

fn run(program: &str, args: &[String]) -> i32 {
    let said = |why: String| {
        eprintln!("tirith: {why}");
        71
    };
    let as_it_is = || {
        said(format!(
            "cannot start {program}: {}",
            std::process::Command::new(program).args(args).exec()
        ))
    };
    // SAFETY: getpid only reads this process's own id.
    let me = unsafe { libc::getpid() };
    if !mac::ours(me) {
        return as_it_is(); // outside, Codex's own sandbox works
    }
    let Some(asked) = asked::read(args) else {
        return as_it_is();
    };
    let Ok(cwd) = std::env::current_dir() else {
        return said("this folder is gone".into());
    };
    let home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::config::home_dir().join(".codex"));
    let tmp = std::env::var_os("TMPDIR").map(PathBuf::from);
    let at = settings::Where {
        cwd: &cwd,
        home: &cwd.join(home),
        tmp: tmp.as_deref(),
    };
    let (mut request, note) = match settings::limit(&asked, &at) {
        Ok(Some(limited)) => limited,
        Ok(None) => return as_it_is(), // full access has no sandbox of Codex's to start
        Err(why) => return said(why),
    };
    if mac::tighter(me) {
        return said("a Codex already under a limit starts no other Codex".into());
    }
    let Some(program) = whole(program, &cwd) else {
        return said(format!("no {program} on PATH"));
    };
    request.program = program.display().to_string();
    request.env = std::env::vars_os()
        .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
        .collect();
    eprintln!("tirith: Codex's own sandbox cannot start inside Tirith's on macOS: {note}");
    ask(&request)
        .unwrap_or_else(|why| said(format!("Codex could not be started with its limit: {why}")))
}
