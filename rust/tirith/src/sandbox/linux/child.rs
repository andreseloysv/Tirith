//! The confined side: into its own network, its forwarded ports handed out, then the program.
use super::{handover, namespace};
use std::io::Read;
use std::net::{Ipv4Addr, TcpListener};
use std::os::fd::AsRawFd;
use std::os::unix::{net::UnixStream, process::CommandExt};

/// The exit code of a confinement that could not be made.
const REFUSED: i32 = 125;

/// Runs in the forked child and never returns. With no program it is only the probe of whether this works.
pub(super) fn child(mut link: UnixStream, ports: &[u16], program: &[String], parent: i32) -> ! {
    if let Err(why) = prepare(&mut link, ports, parent) {
        eprintln!("tirith sandbox: {why}");
        std::process::exit(REFUSED);
    }
    drop(link);
    let Some((name, args)) = program.split_first() else {
        std::process::exit(0);
    };
    let why = std::process::Command::new(name).args(args).exec();
    eprintln!("tirith sandbox: cannot start {name}: {why}");
    std::process::exit(127);
}

fn prepare(link: &mut UnixStream, ports: &[u16], parent: i32) -> Result<(), String> {
    namespace::enter()?;
    // After the namespaces (changing them clears it): it never outlives the launcher, its only way out.
    unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) };
    (unsafe { libc::getppid() } == parent)
        .then_some(())
        .ok_or("the launcher is gone")?;
    let inside = ports
        .iter()
        .map(|p| TcpListener::bind((Ipv4Addr::LOCALHOST, *p)).map_err(|e| format!("port {p}: {e}")))
        .collect::<Result<Vec<_>, _>>()?;
    let fds: Vec<_> = inside.iter().map(AsRawFd::as_raw_fd).collect();
    handover::send(link, &fds)?;
    link.read_exact(&mut [0u8])
        .map_err(|e| format!("the launcher did not take the ports: {e}"))
}
