//! The WebSocket frames (RFC 6455) of the bridge: one peer on this machine, whole messages, no extensions.
use std::io::{Read, Result, Write};

/// The most one message holds, a whole file as JSON among them: what Codex's own WebSocket takes (over it Codex drops its connection, and the run with it). The daemon holds one of them each way.
pub(super) const MOST: u64 = 16 << 20;
pub(super) const TEXT: u8 = 1;
pub(super) const CLOSE: u8 = 8;
const PING: u8 = 9;
pub(super) const PONG: u8 = 10;

/// One frame from the server's side, which is never masked.
pub(super) fn send(to: &mut impl Write, op: u8, data: &[u8]) -> Result<()> {
    let mut head = vec![0x80 | op];
    match data.len() {
        n if n < 126 => head.push(n as u8),
        n if n <= usize::from(u16::MAX) => {
            head.push(126);
            head.extend((n as u16).to_be_bytes());
        }
        n => {
            head.push(127);
            head.extend((n as u64).to_be_bytes());
        }
    }
    to.write_all(&head)?;
    to.write_all(data)?;
    to.flush()
}

/// What the client sent.
pub(super) enum Got {
    Whole(Vec<u8>),
    /// More than `MOST`: its beginning, the rest read and dropped.
    TooBig(Vec<u8>),
}

/// The next message of the client; none at its close, its end or what is no such frame. A ping is handed to `pong`.
pub(super) fn next(from: &mut impl Read, mut pong: impl FnMut(&[u8])) -> Option<Got> {
    let (mut whole, mut big) = (vec![], false);
    loop {
        let [first, second] = exactly::<2>(from)?;
        let (last, op) = (first & 0x80 != 0, first & 0x0f);
        (second & 0x80 != 0).then_some(())?; // a client always masks
        let size = match second & 0x7f {
            126 => u64::from(u16::from_be_bytes(exactly::<2>(from)?)),
            127 => u64::from_be_bytes(exactly::<8>(from)?),
            n => u64::from(n),
        };
        let mask = exactly::<4>(from)?;
        let message = op <= 2;
        // a control frame holds 125 bytes at most
        (message || size <= 125).then_some(())?;
        let room = if message {
            MOST - whole.len() as u64
        } else {
            size
        };
        let (kept, at) = (size.min(room), if message { whole.len() } else { 0 });
        let mut control = vec![];
        let into = if message { &mut whole } else { &mut control };
        from.take(kept).read_to_end(into).ok()?;
        ((into.len() - at) as u64 == kept).then_some(())?;
        for (i, byte) in into[at..].iter_mut().enumerate() {
            *byte ^= mask[i % 4];
        }
        if kept < size {
            big = true;
            let dropped = std::io::copy(&mut from.take(size - kept), &mut std::io::sink()).ok()?;
            (dropped == size - kept).then_some(())?;
        }
        match op {
            0..=2 if last && big => return Some(Got::TooBig(whole)),
            0..=2 if last => return Some(Got::Whole(whole)),
            0..=2 => {}
            PING => pong(&control),
            PONG => {}
            _ => return None,
        }
    }
}

fn exactly<const N: usize>(from: &mut impl Read) -> Option<[u8; N]> {
    let mut bytes = [0u8; N];
    from.read_exact(&mut bytes).ok()?;
    Some(bytes)
}
