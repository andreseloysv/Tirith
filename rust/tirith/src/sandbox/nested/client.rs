//! `codex` inside Tirith's sandbox (the wrapper of `<home>/net/bin`): its limit is read and the daemon is asked to apply it.
use super::{ask::ask, asked, other, program, settings, ROLE, WORD};
use crate::sandbox::mac;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;

/// `tirith nested-codex <codex> args…`, answered before the command line is read.
pub fn launcher(args: &[String]) -> Option<i32> {
    let [word, program, rest @ ..] = args else {
        return None;
    };
    (word == WORD || word == ROLE).then(|| run(program, rest, word == ROLE))
}

/// `argv` of a Codex that Tirith starts for a read-only role inside its sandbox, through this client.
pub fn inside(argv: &mut Vec<String>) {
    if let Some(exe) = crate::app::exe() {
        argv.splice(0..0, [exe.display().to_string(), ROLE.to_string()]);
    }
}

fn run(program: &str, args: &[String], role: bool) -> i32 {
    let said = |why: String| {
        eprintln!("tirith: Codex could not be started with its limit: {why}");
        71
    };
    let as_it_is = || {
        let e = std::process::Command::new(program).args(args).exec();
        eprintln!("tirith: cannot start {program}: {e}");
        71
    };
    // SAFETY: getpid only reads this process's own id.
    let me = unsafe { libc::getpid() };
    if !mac::ours(me) {
        return as_it_is(); // outside, Codex's own sandbox works
    }
    let Some(asked) = asked::read(args) else {
        match other::said(args) {
            Some((why, true)) => return said(why),
            Some((note, false)) => eprintln!("tirith: {note}"),
            None => {}
        }
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
        role,
    };
    let (mut request, note) = match settings::limit(&asked, &at) {
        Ok(Some(limited)) => limited,
        Ok(None) => return as_it_is(), // full access has no sandbox of Codex's to start
        Err(why) => return said(why),
    };
    if mac::tighter(me) {
        return said("a Codex already under a limit starts no other Codex".into());
    }
    let Some(found) = program::whole(program, &cwd) else {
        return said(format!("no {program} on PATH"));
    };
    let Some(binary) = program::native(&found) else {
        return said(format!(
            "{} is a script and Codex's own binary is not beside it: put that binary first on PATH",
            found.display()
        ));
    };
    request.program = binary.display().to_string();
    request.env = std::env::vars_os()
        .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
        .collect();
    eprintln!("tirith: {note}");
    ask(&request).unwrap_or_else(said)
}
