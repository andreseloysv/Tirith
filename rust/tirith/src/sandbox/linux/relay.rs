//! The only ways out of a confined process: its loopback ports that reach the same port of this machine (the daemon, its proxy).
use std::io::copy;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread::Builder;

/// A relay's stack: it only copies.
const STACK: usize = 256 * 1024;

/// Serves `inside` (a listener in the confined network) from the launcher's network: each caller gets 127.0.0.1:`port` here, from the sandbox's own address (`source.rs`).
pub(super) fn serve(inside: TcpListener, port: u16) {
    let work = move || {
        for caller in inside.incoming() {
            match caller {
                Ok(caller) => {
                    let _ = Builder::new()
                        .stack_size(STACK)
                        .spawn(move || join(caller, port));
                }
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(50)),
            }
        }
    };
    let _ = Builder::new().stack_size(STACK).spawn(work);
}

/// The bytes both ways until both sides closed.
fn join(caller: TcpStream, port: u16) {
    let Some(there) = (0..3).find_map(|_| super::source::connect(port)) else {
        return;
    };
    let _ = (caller.set_nodelay(true), there.set_nodelay(true));
    let (Ok(caller2), Ok(there2)) = (caller.try_clone(), there.try_clone()) else {
        return;
    };
    let back = Builder::new()
        .stack_size(STACK)
        .spawn(move || pump(there2, caller2));
    pump(caller, there);
    let _ = back.map(|h| h.join());
}

fn pump(mut from: TcpStream, mut to: TcpStream) {
    let _ = copy(&mut from, &mut to);
    let _ = to.shutdown(Shutdown::Write);
}
