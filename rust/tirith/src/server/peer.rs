//! Who is at the other end of a loopback connection: on Linux the kernel's table of sockets names each one's user.
use crate::app::Shared;
use std::net::SocketAddr;
use tokio::net::TcpStream;

/// True when the caller may use this OpenBot. An unknown caller may not.
#[cfg(target_os = "linux")]
pub(super) fn welcome(app: &Shared, stream: &TcpStream, from: SocketAddr) -> bool {
    let Ok(to) = stream.local_addr() else {
        return false;
    };
    let table = |name: &str| std::fs::read_to_string(format!("/proc/self/net/{name}")).ok();
    let found = ["tcp", "tcp6"]
        .iter()
        .find_map(|name| uid(&table(name)?, from, to));
    let Some(uid) = found else {
        return false;
    };
    // SAFETY: geteuid only reads this process's own user.
    if uid == 0 || uid == unsafe { libc::geteuid() } {
        return true;
    }
    let users: Vec<String> =
        crate::config::get(&app.cfg().raw, &["server", "users"]).unwrap_or_default();
    let passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    users.iter().any(|user| is(&passwd, user, uid))
}

/// Elsewhere the loopback is the owner's alone: nothing says who called.
#[cfg(not(target_os = "linux"))]
pub(super) fn welcome(_app: &Shared, _stream: &TcpStream, _from: SocketAddr) -> bool {
    true
}

/// The user of the socket `from` -> `to` in a table like /proc/net/tcp. A socket its process closed has no inode, and the kernel calls it root's: that one is nobody.
#[cfg(any(test, target_os = "linux"))]
pub(super) fn uid(table: &str, from: SocketAddr, to: SocketAddr) -> Option<u32> {
    let (from, to) = (shapes(from)?, shapes(to)?);
    table.lines().skip(1).find_map(|line| {
        let part: Vec<&str> = line.split_whitespace().collect();
        let ours = part.len() > 9 && from.iter().any(|f| f == part[1]);
        let ours = ours && to.iter().any(|t| t == part[2]) && part[9] != "0";
        ours.then(|| part[7].parse().ok()).flatten()
    })
}

/// An IPv4 address and port as the table writes it, and as its IPv6 twin does for a dual-stack socket.
#[cfg(any(test, target_os = "linux"))]
fn shapes(at: SocketAddr) -> Option<[String; 2]> {
    let SocketAddr::V4(at) = at else { return None };
    let four = format!("{:08X}", u32::from_ne_bytes(at.ip().octets()));
    let port = format!("{:04X}", at.port());
    let six = format!("0000000000000000FFFF0000{four}:{port}");
    Some([format!("{four}:{port}"), six])
}

/// True when `user`, a name in passwd or a number, is `uid`.
#[cfg(any(test, target_os = "linux"))]
pub(super) fn is(passwd: &str, user: &str, uid: u32) -> bool {
    let named = passwd.lines().any(|line| {
        let mut part = line.split(':');
        part.next() == Some(user) && part.nth(1).and_then(|n| n.parse().ok()) == Some(uid)
    });
    named || user.parse() == Ok(uid)
}
