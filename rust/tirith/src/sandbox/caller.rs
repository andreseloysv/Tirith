//! Whether a connection to the daemon comes from inside NeoBot's sandbox, as the kernel tells it: on Linux by the address only the sandbox's relay calls from, on macOS by the profile of the process that holds the socket.
use std::net::SocketAddr;

/// True when the caller at `from`, connected to `to`, runs confined.
#[cfg(target_os = "linux")]
pub fn confined(from: SocketAddr, _to: SocketAddr) -> bool {
    from.ip().to_canonical() == std::net::IpAddr::V4(super::linux::FROM)
}

/// True when the caller at `from`, connected to `to`, runs confined, or cannot be told (it left, or it is another user's).
#[cfg(target_os = "macos")]
pub fn confined(from: SocketAddr, to: SocketAddr) -> bool {
    super::mac::confined(from.port(), to.port())
}

/// Elsewhere there is no sandbox to come from.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn confined(_from: SocketAddr, _to: SocketAddr) -> bool {
    false
}
