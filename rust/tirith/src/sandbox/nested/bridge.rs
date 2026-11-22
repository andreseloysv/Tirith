//! The one way to the process that runs a limited Codex's commands and patches (`exec-server --listen stdio`): a WebSocket on loopback that takes one connection, of that Codex's own process as the kernel tells, and hands each message over that process's pipes. Nobody else reaches it: it listens nowhere. The bridge keeps its port for as long as the run lives, so nobody can stand in its place when Codex comes back for another connection.
use super::carry::carry;
use crate::sandbox::mac;
use std::net::TcpListener;
use std::process::{ChildStdin, ChildStdout};
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::Arc;
use std::time::Duration;

#[derive(Default)]
pub(super) struct Seen {
    linked: AtomicBool,
    over: AtomicBool,
    /// Codex's end went first: it is leaving.
    pub left: AtomicBool,
    /// The other went first: nothing can run any more.
    pub lost: AtomicBool,
    /// A call was over the cap and was refused.
    pub cut: AtomicBool,
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
            let mut pipes = Some((to, from));
            while !seen.over.load(SeqCst) {
                let Ok((peer, at)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(15));
                    continue;
                };
                // one connection, of its own process: any later one is dropped, and the port stays the bridge's
                if pipes.is_none() || !mac::held_by(at.port(), port, codex) {
                    continue;
                }
                if let Some((to, from)) = pipes.take() {
                    seen.linked.store(true, SeqCst);
                    let seen = seen.clone();
                    std::thread::spawn(move || carry(peer, to, from, seen));
                }
            }
        });
    }

    /// Whether that Codex handed its commands over.
    pub(super) fn linked(&self) -> bool {
        self.seen.linked.load(SeqCst)
    }

    /// Whether what runs its commands is gone while Codex is not.
    pub(super) fn lost(&self) -> bool {
        self.seen.lost.load(SeqCst)
    }

    /// Whether a call was refused for its size since this was last asked.
    pub(super) fn cut(&self) -> bool {
        self.seen.cut.swap(false, SeqCst)
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.seen.over.store(true, SeqCst);
    }
}
