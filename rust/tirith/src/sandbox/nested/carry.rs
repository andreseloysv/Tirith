//! The bridge's two ways, once Codex's own process is on it.
use super::{frames, upgrade::upgrade};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::process::{ChildStdin, ChildStdout};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Messages both ways, until either end is gone.
pub(super) fn carry(mut peer: TcpStream, mut to: ChildStdin, from: ChildStdout) -> Option<()> {
    peer.set_nonblocking(false).ok()?;
    peer.set_read_timeout(Some(Duration::from_secs(10))).ok()?;
    upgrade(&mut peer)?;
    peer.set_read_timeout(None).ok()?;
    let writer = Arc::new(Mutex::new(peer.try_clone().ok()?));
    let back = writer.clone();
    std::thread::spawn(move || {
        let mut lines = BufReader::new(from);
        let mut line = vec![];
        while let Ok(1..) = (&mut lines).take(frames::MOST).read_until(b'\n', &mut line) {
            let message = line.trim_ascii();
            let mut out = back.lock().unwrap_or_else(|e| e.into_inner());
            if !message.is_empty() && frames::send(&mut *out, frames::TEXT, message).is_err() {
                break;
            }
            line.clear();
        }
        let mut out = back.lock().unwrap_or_else(|e| e.into_inner());
        let _ = frames::send(&mut *out, frames::CLOSE, &[]);
        let _ = out.shutdown(Shutdown::Both);
    });
    let pong = |data: &[u8]| {
        let mut out = writer.lock().unwrap_or_else(|e| e.into_inner());
        let _ = frames::send(&mut *out, frames::PONG, data);
    };
    while let Some(mut message) = frames::next(&mut peer, pong) {
        // a line each: JSON's own line breaks are only spaces
        message
            .iter_mut()
            .filter(|b| **b == b'\n')
            .for_each(|b| *b = b' ');
        message.push(b'\n');
        to.write_all(&message).ok()?;
    }
    Some(())
}
