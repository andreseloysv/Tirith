//! Where the relay calls from: an address of the machine's loopback that is the sandbox's alone, so the daemon tells a confined caller whatever it sends.
use std::net::{Ipv4Addr, TcpStream};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

pub(in crate::sandbox) const FROM: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 2);
/// `IP_BIND_ADDRESS_NO_PORT`: the port is chosen on connecting, as an unbound socket's is.
const NO_PORT: libc::c_int = 24;

fn address(ip: Ipv4Addr, port: u16) -> libc::sockaddr_in {
    libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: port.to_be(),
        sin_addr: libc::in_addr {
            s_addr: u32::from_ne_bytes(ip.octets()),
        },
        sin_zero: [0; 8],
    }
}

/// A connection from `FROM` to this machine's 127.0.0.1:`port`.
pub(super) fn connect(port: u16) -> Option<TcpStream> {
    let size = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
    let (here, there) = (address(FROM, 0), address(Ipv4Addr::LOCALHOST, port));
    // SAFETY: the addresses and the option live through the calls, with their sizes; the new socket has one owner.
    unsafe {
        let fd = libc::socket(libc::AF_INET, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0);
        let sock = (fd >= 0).then(|| OwnedFd::from_raw_fd(fd))?;
        let (fd, on): (_, libc::c_int) = (sock.as_raw_fd(), 1);
        let len = std::mem::size_of_val(&on) as libc::socklen_t;
        libc::setsockopt(fd, libc::IPPROTO_IP, NO_PORT, (&raw const on).cast(), len);
        (libc::bind(fd, (&raw const here).cast(), size) == 0).then_some(())?;
        (libc::connect(fd, (&raw const there).cast(), size) == 0).then_some(())?;
        Some(TcpStream::from(sock))
    }
}
