//! A watch on what the broker started, outliving the daemon: when Tirith stops in the middle, also by force, that Codex and all below it are ended, never left running with nobody to answer to. The watcher is Tirith's own program with none of the caller's environment: it runs outside every sandbox.
use super::ending::end_all;
use libc::pid_t;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// The word Tirith's program is the watcher by.
const WORD: &str = "nested-codex-watch";
const LOOK: Duration = Duration::from_millis(250);

pub(super) struct Watch(Child);

impl Watch {
    /// Over process `pid`, for as long as this value lives.
    pub(super) fn over(pid: u32) -> Result<Watch, String> {
        let exe = crate::app::exe().ok_or("Tirith's program is not known")?;
        Command::new(exe)
            .args([
                WORD.to_string(),
                std::process::id().to_string(),
                pid.to_string(),
            ])
            .env_clear()
            .current_dir("/")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .map(Watch)
            .map_err(|e| format!("cannot watch over it: {e}"))
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        let _ = self.0.kill(); // it is over, and waited for: nothing to watch
        let _ = self.0.wait();
    }
}

/// `tirith nested-codex-watch <daemon> <pid>`.
pub(super) fn launcher(args: &[String]) -> Option<i32> {
    let [word, daemon, pid] = args else {
        return None;
    };
    (word == WORD).then_some(())?;
    Some(match (daemon.parse(), pid.parse()) {
        (Ok(daemon), Ok(pid)) if pid > 1 => run(daemon, pid),
        _ => 2,
    })
}

fn run(daemon: pid_t, pid: pid_t) -> i32 {
    loop {
        // SAFETY: both only read; a process group's leader is the one the broker started, whose group it made.
        let (parent, group) = unsafe { (libc::getppid(), libc::getpgid(pid)) };
        if group != pid {
            return 0; // it is gone
        }
        if parent != daemon {
            end_all(pid);
            return 0;
        }
        std::thread::sleep(LOOK);
    }
}
