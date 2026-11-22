//! The bridge's two ways, once Codex's own process is on it. Whichever end is gone first says whether the run is lost.
use super::{answers::answers, bridge::Seen, frames, too_big::refusal, upgrade::upgrade};
use std::io::Write;
use std::net::{Shutdown, TcpStream};
use std::process::{ChildStdin, ChildStdout};
use std::sync::{atomic::Ordering::SeqCst, Arc, Mutex};
use std::time::Duration;

pub(super) type Out = Arc<Mutex<TcpStream>>;

pub(super) fn send(out: &Out, op: u8, data: &[u8]) -> std::io::Result<()> {
    frames::send(
        &mut *out.lock().unwrap_or_else(|e| e.into_inner()),
        op,
        data,
    )
}

/// Messages both ways, until either end is gone: `left` when Codex's was first, `lost` when the other was.
pub(super) fn carry(
    mut peer: TcpStream,
    to: ChildStdin,
    from: ChildStdout,
    seen: Arc<Seen>,
) -> Option<()> {
    peer.set_nonblocking(false).ok()?;
    peer.set_read_timeout(Some(Duration::from_secs(10))).ok()?;
    upgrade(&mut peer)?;
    peer.set_read_timeout(None).ok()?;
    let out: Out = Arc::new(Mutex::new(peer.try_clone().ok()?));
    let (back, over) = (out.clone(), seen.clone());
    std::thread::spawn(move || {
        answers(from, &back);
        if !over.left.load(SeqCst) {
            over.lost.store(true, SeqCst); // before Codex can come back for another
        }
        let _ = send(&back, frames::CLOSE, &[]);
        let _ = back
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .shutdown(Shutdown::Both);
    });
    asks(&mut peer, to, &out, &seen)
}

/// What Codex asks, until it leaves.
fn asks(peer: &mut TcpStream, mut to: ChildStdin, out: &Out, seen: &Seen) -> Option<()> {
    let pong = |data: &[u8]| drop(send(out, frames::PONG, data));
    loop {
        let Some(got) = frames::next(peer, pong) else {
            seen.left.store(true, SeqCst); // before `to` closes and ends the other
            return Some(());
        };
        let mut message = match got {
            frames::Got::Whole(message) => message,
            frames::Got::TooBig(start) => match refusal(&start) {
                Some(error) => {
                    send(out, frames::TEXT, &error).ok()?;
                    continue;
                }
                None => {
                    seen.lost.store(true, SeqCst);
                    return None;
                }
            },
        };
        // a line each: JSON's own line breaks are only spaces
        message
            .iter_mut()
            .filter(|b| **b == b'\n')
            .for_each(|b| *b = b' ');
        message.push(b'\n');
        if to.write_all(&message).is_err() {
            seen.lost.store(true, SeqCst);
            return None;
        }
    }
}
