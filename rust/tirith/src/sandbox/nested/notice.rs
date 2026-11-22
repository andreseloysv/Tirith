//! What the daemon says to a caller while its Codex runs, on that caller's own stderr: Codex's report of a failed patch drops the reason it was given.
use std::io::Write;
use std::os::fd::OwnedFd;

pub(super) struct Notice(Option<std::fs::File>);

impl Notice {
    pub(super) fn to(stderr: &OwnedFd) -> Notice {
        Notice(stderr.try_clone().ok().map(std::fs::File::from))
    }

    pub(super) fn say(&mut self, why: &str) {
        if let Some(out) = self.0.as_mut() {
            let _ = writeln!(out, "tirith: {why}");
        }
    }
}
