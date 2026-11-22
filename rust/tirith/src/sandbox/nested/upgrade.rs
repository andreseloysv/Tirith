//! The bridge's half of a WebSocket's opening. A web page's is refused: it says where it comes from.
use base64::Engine;
use std::io::{Read, Write};

const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const MOST: usize = 16 * 1024;

extern "C" {
    fn CC_SHA1(data: *const libc::c_void, len: u32, out: *mut u8) -> *mut u8;
}

/// What a server answers to a client's key.
pub(super) fn accepted(key: &str) -> String {
    let text = format!("{key}{GUID}");
    let mut out = [0u8; 20];
    // SAFETY: CommonCrypto (libSystem) reads `len` bytes and writes the 20 of a SHA-1.
    unsafe { CC_SHA1(text.as_ptr().cast(), text.len() as u32, out.as_mut_ptr()) };
    base64::engine::general_purpose::STANDARD.encode(out)
}

/// The client's opening answered; none for what is not one.
pub(super) fn upgrade(peer: &mut (impl Read + Write)) -> Option<()> {
    let mut head = vec![];
    while !head.ends_with(b"\r\n\r\n") {
        let mut byte = [0u8];
        peer.read_exact(&mut byte).ok()?;
        head.push(byte[0]);
        (head.len() <= MOST).then_some(())?;
    }
    let head = String::from_utf8(head).ok()?;
    let header = |name: &str| {
        head.lines().skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case(name)
                .then(|| value.trim().to_string())
        })
    };
    header("origin").is_none().then_some(())?;
    let key = header("sec-websocket-key")?;
    let answer = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
        accepted(&key)
    );
    peer.write_all(answer.as_bytes()).ok()
}
