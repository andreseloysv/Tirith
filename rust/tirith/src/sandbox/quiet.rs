//! `tirith quiet`: an agent that was given private values (a token, an enterprise's host) starts through it, and they are hidden in all it writes before that reaches a log, a chat or an event.
use crate::setup::Hidden;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc};
use std::time::Duration;

const WORD: &str = "quiet";

/// `program args…` as it starts when `vars` hold something private. Their names are on the command line, never their values.
pub(super) fn argv(
    vars: &[(String, String)],
    program: String,
    args: Vec<String>,
) -> (String, Vec<String>) {
    if Hidden::of(vars).none() {
        return (program, args);
    }
    let Some(exe) = crate::app::exe() else {
        return (program, args);
    };
    let names: Vec<&str> = vars.iter().map(|(name, _)| name.as_str()).collect();
    let mut all = vec![WORD.to_string(), names.join(","), "--".to_string(), program];
    all.extend(args);
    (exe.display().to_string(), all)
}

/// The launchers of what OpenBot starts, answered before the command line is read: this one, then the sandbox's.
pub fn launcher(args: &[String]) -> Option<i32> {
    match args {
        [word, names, dashes, program, rest @ ..] if word == WORD && dashes == "--" => {
            Some(run(names, program, rest))
        }
        _ => {
            #[cfg(target_os = "macos")]
            if let Some(code) = super::nested::launcher(args) {
                return Some(code);
            }
            super::confine::launcher(args)
        }
    }
}

fn run(names: &str, program: &str, args: &[String]) -> i32 {
    let value = |name: &str| std::env::var(name).ok().map(|v| (name.to_string(), v));
    let vars: Vec<(String, String)> = names.split(',').filter_map(value).collect();
    let hidden = Arc::new(Hidden::of(&vars));
    let mut command = Command::new(program);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            eprintln!("tirith: cannot start {program}: {e}");
            return 127;
        }
    };
    let (done, ends) = mpsc::channel();
    if let (Some(out), Some(err)) = (child.stdout.take(), child.stderr.take()) {
        copy(out, std::io::stdout(), hidden.clone(), done.clone());
        copy(err, std::io::stderr(), hidden, done);
    }
    let ended = child.wait();
    for _ in 0..2 {
        let _ = ends.recv_timeout(Duration::from_secs(2)); // what it left behind may hold them open
    }
    ended.map_or(125, |s| s.code().unwrap_or(128 + s.signal().unwrap_or(0)))
}

/// Every line of `from`, hidden, on `to`.
fn copy<R, W>(from: R, mut to: W, hidden: Arc<Hidden>, done: mpsc::Sender<()>)
where
    R: Read + Send + 'static,
    W: Write + Send + 'static,
{
    std::thread::spawn(move || {
        let (mut from, mut line) = (BufReader::new(from), vec![]);
        while from.read_until(b'\n', &mut line).is_ok_and(|n| n > 0) {
            let shown = hidden.hide(&String::from_utf8_lossy(&line));
            if to
                .write_all(shown.as_bytes())
                .and_then(|_| to.flush())
                .is_err()
            {
                break;
            }
            line.clear();
        }
        let _ = done.send(());
    });
}
