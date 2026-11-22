//! Reading a secret: one line of stdin, not shown as it is typed, never on a command line.
use std::io::{BufRead, IsTerminal};

pub fn read(prompt: &str) -> Result<String, String> {
    let stdin = std::io::stdin();
    let hidden = stdin.is_terminal().then(Hidden::new).flatten();
    if hidden.is_some() {
        eprint!("{prompt}: ");
    }
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    if hidden.is_some() {
        eprintln!();
    }
    drop(hidden);
    let word = line.trim().to_string();
    (!word.is_empty())
        .then_some(word)
        .ok_or_else(|| "nothing came on stdin".to_string())
}

/// The terminal's echo off for as long as it lives.
struct Hidden(libc::termios);

impl Hidden {
    fn new() -> Option<Hidden> {
        unsafe {
            let mut was: libc::termios = std::mem::zeroed();
            (libc::tcgetattr(0, &mut was) == 0).then_some(())?;
            let mut now = was;
            now.c_lflag &= !libc::ECHO;
            (libc::tcsetattr(0, libc::TCSANOW, &now) == 0).then_some(Hidden(was))
        }
    }
}

impl Drop for Hidden {
    fn drop(&mut self) {
        unsafe { libc::tcsetattr(0, libc::TCSANOW, &self.0) };
    }
}
