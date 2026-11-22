//! The one way to the process that runs a limited Codex's commands and patches (`exec-server --listen stdio`): a WebSocket on loopback that takes one connection, of that Codex's own process as the kernel tells, and hands each message over that process's pipes. Nobody else reaches it: it listens nowhere.
use super::carry::carry;
use crate::sandbox::mac;
use std::net::TcpListener;
use std::process::{ChildStdin, ChildStdout};
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::Arc;
use std::time::Duration;

#[derive(Default)]
struct Seen {
    linked: AtomicBool,
    over: AtomicBool,
}

pub(super) struct Bridge {
    listener: Option<TcpListener>,
    pub port: u16,
    seen: Arc<Seen>,
}

impl Bridge {
    pub(super) fn open() -> Result<Bridge, String> {
        let open = || -> std::io::Result<Bridge> {
            let listener = TcpListener::bind(("127.0.0.1", 0))?;
            listener.set_nonblocking(true)?;
            Ok(Bridge {
                port: listener.local_addr()?.port(),
                listener: Some(listener),
                seen: Arc::default(),
            })
        };
        open().map_err(|e| format!("no bridge to what runs its commands: {e}"))
    }

    /// For process `codex` alone, to those pipes.
    pub(super) fn serve(&mut self, codex: libc::pid_t, to: ChildStdin, from: ChildStdout) {
        let Some(listener) = self.listener.take() else {
            return;
        };
        let (seen, port) = (self.seen.clone(), self.port);
        std::thread::spawn(move || {
            while !seen.over.load(SeqCst) {
                let Ok((peer, at)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(15));
                    continue;
                };
                if mac::held_by(at.port(), port, codex) {
                    drop(listener); // one connection, its own
                    seen.linked.store(true, SeqCst);
                    carry(peer, to, from);
                    return;
                }
            }
        });
    }

    /// Whether that Codex handed its commands over.
    pub(super) fn linked(&self) -> bool {
        self.seen.linked.load(SeqCst)
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.seen.over.store(true, SeqCst);
    }
}
