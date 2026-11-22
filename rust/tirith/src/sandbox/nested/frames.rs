//! The WebSocket frames (RFC 6455) of the bridge: one peer on this machine, whole messages, no extensions.
use std::io::{Read, Result, Write};

/// The most one message holds (a file read whole, as JSON).
pub(super) const MOST: u64 = 1 << 30;
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

/// The next whole message of the client; none at its close, its end or what is no such frame. A ping is handed to `pong`.
pub(super) fn next(from: &mut impl Read, mut pong: impl FnMut(&[u8])) -> Option<Vec<u8>> {
    let mut whole = vec![];
    loop {
        let [first, second] = exactly::<2>(from)?;
        let (last, op) = (first & 0x80 != 0, first & 0x0f);
        (second & 0x80 != 0).then_some(())?; // a client always masks
        let size = match second & 0x7f {
            126 => u64::from(u16::from_be_bytes(exactly::<2>(from)?)),
            127 => u64::from_be_bytes(exactly::<8>(from)?),
            n => u64::from(n),
        };
        (size <= MOST && size + whole.len() as u64 <= MOST).then_some(())?;
        let mask = exactly::<4>(from)?;
        let mut data = vec![];
        from.take(size).read_to_end(&mut data).ok()?;
        (data.len() as u64 == size).then_some(())?;
        for (i, byte) in data.iter_mut().enumerate() {
            *byte ^= mask[i % 4];
        }
        match op {
            0..=2 => {
                whole.append(&mut data);
                if last {
                    return Some(whole);
                }
            }
            PING => pong(&data),
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
