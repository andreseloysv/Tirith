//! The client's talk with the broker: its own stdio and the request, then the exit code of what ran. Nothing is handed to a socket a confined process put in the daemon's place: what is handed holds the caller's environment.
use super::{place, Request};
use crate::sandbox::{handover, mac};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

pub(super) fn ask(request: &Request) -> Result<i32, String> {
    let path = place::socket(&crate::config::tirith_home()).ok_or("no folder for its socket")?;
    let mut s = UnixStream::connect(&path)
        .map_err(|e| format!("OpenBot's daemon does not answer ({e})"))?;
    let server = place::peer(&s).ok_or("that socket is not this user's")?;
    if mac::ours(server) {
        return Err("what holds the socket is not OpenBot's daemon (it is confined itself)".into());
    }
    handover::send(&s, &[0, 1, 2])?;
    let line = serde_json::to_string(request).map_err(|e| e.to_string())? + "\n";
    s.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
    let mut answer = String::new();
    BufReader::new(&s)
        .read_line(&mut answer)
        .map_err(|e| e.to_string())?;
    let answer = answer.trim();
    if answer.is_empty() {
        return Err("OpenBot stopped while it ran".into());
    }
    answer.parse().map_err(|_| answer.to_string())
}
