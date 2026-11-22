//! Who a connection to the daemon comes from, as the kernel tells it: whether from inside Tirith's sandbox, and on macOS whether from a command under its caller's own limit there (`nested/`). On Linux by the address only the sandbox's relay calls from, on macOS by the profile of the process that holds the socket.
use std::net::SocketAddr;

/// (confined, limited) of the caller at `from`, connected to `to`.
#[cfg(target_os = "linux")]
pub fn who(from: SocketAddr, _to: SocketAddr) -> (bool, bool) {
    let confined = from.ip().to_canonical() == std::net::IpAddr::V4(super::linux::FROM);
    (confined, false) // Codex's own sandbox holds there
}

/// (confined, limited) of the caller at `from`, connected to `to`; both, when it cannot be told (it left, or it is another user's).
#[cfg(target_os = "macos")]
pub fn who(from: SocketAddr, to: SocketAddr) -> (bool, bool) {
    super::mac::who(from.port(), to.port())
}

/// Elsewhere there is no sandbox to come from.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn who(_from: SocketAddr, _to: SocketAddr) -> (bool, bool) {
    (false, false)
}

/// Whether that caller runs confined.
#[cfg(test)]
pub fn confined(from: SocketAddr, to: SocketAddr) -> bool {
    who(from, to).0
}
