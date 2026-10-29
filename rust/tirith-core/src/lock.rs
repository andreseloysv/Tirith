//! One continuous collector per Tirith home, enforced by an advisory file lock (`collectlock.py`).
//!
//! The lock lives on `<home>/collector.lock`, is held for the life of the process and dies with it, so a
//! crash never leaves a stale "running" answer. The file holds the pid of the holder.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub const LOCK_NAME: &str = "collector.lock";

/// Another collector holds the lock; `pid` is its pid when the file says so.
#[derive(Debug)]
pub struct CollectorRunning {
    pub pid: Option<u32>,
}

impl std::fmt::Display for CollectorRunning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.pid {
            Some(pid) => write!(f, "a collector is already running, pid {pid}"),
            None => write!(f, "a collector is already running, pid ?"),
        }
    }
}

/// Holds the lock until dropped.
#[derive(Debug)]
pub struct CollectorLock {
    _file: File,
}

/// Take the collector lock, or say who has it.
pub fn hold_collector_lock(home: &Path) -> Result<CollectorLock, CollectorRunning> {
    let busy = |pid| CollectorRunning { pid };
    fs::create_dir_all(home).map_err(|_| busy(None))?;
    let mut file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(home.join(LOCK_NAME))
        .map_err(|_| busy(None))?;
    match file.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) | Err(TryLockError::Error(_)) => {
            let mut text = String::new();
            let _ = file.seek(SeekFrom::Start(0));
            let _ = file.read_to_string(&mut text);
            return Err(busy(text.trim().parse().ok()));
        }
    }
    file.set_len(0).map_err(|_| busy(None))?;
    // append mode writes at the end, which is the start of the emptied file
    file.write_all(format!("{}\n", std::process::id()).as_bytes()).map_err(|_| busy(None))?;
    file.flush().map_err(|_| busy(None))?;
    Ok(CollectorLock { _file: file })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_holder_is_refused_until_the_first_drops() {
        let dir = tempfile::tempdir().unwrap();
        let first = hold_collector_lock(dir.path()).unwrap();
        let err = hold_collector_lock(dir.path()).unwrap_err();
        assert_eq!(err.pid, Some(std::process::id()));
        drop(first);
        assert!(hold_collector_lock(dir.path()).is_ok());
    }
}
