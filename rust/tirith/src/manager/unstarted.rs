//! A routine's run whose agent does not start is tried a few times and then has failed (the owner's own words keep being tried), and one handed to a session that died without a word has failed too.
use super::talk::talk;
use crate::app::{kill_group, Shared};

/// How often a run's agent is tried.
const TRIES: u32 = 3;

/// A session of `project` ended with `result`; `failed` counts the waiting run's failed starts.
pub(super) fn after(app: &Shared, project: &str, result: &Result<(), String>, failed: &mut u32) {
    let (waiting, taken) = talk(project, |t| {
        let run = t.pending.iter().find(|m| m.job).map(|m| m.by.1.clone());
        (run, t.taken)
    });
    match (result, waiting) {
        (Err(e), Some(routine)) => {
            *failed += 1;
            if *failed < TRIES {
                return;
            }
            *failed = 0;
            talk(project, |t| {
                if let Some(at) = t.pending.iter().position(|m| m.job) {
                    t.pending.remove(at);
                }
            });
            crate::expected::unstarted(app, project, &routine, e);
        }
        (_, None) => {
            *failed = 0;
            if !taken && crate::turns::idle(project) {
                crate::expected::orphaned(app, project);
            }
        }
        (Ok(()), Some(_)) => {}
    }
}

/// True when all that waits is a routine's run: its failure is said once, by `expected/`.
pub(super) fn only_runs(project: &str) -> bool {
    talk(project, |t| {
        !t.pending.is_empty() && t.pending.iter().all(|m| m.job)
    })
}

/// The waiting run of `routine` never starts now.
pub fn drop_run(project: &str, routine: &str) {
    talk(project, |t| {
        if let Some(at) = t.jobs.iter().position(|j| j.by.1 == routine) {
            t.jobs.remove(at);
        } else if let Some(at) = t.pending.iter().position(|m| m.job && m.by.1 == routine) {
            t.pending.remove(at);
        }
    });
}

/// Ends the run that has `project`'s turn; false when none has one.
pub fn stop_run(project: &str) -> bool {
    let pid = talk(project, |t| t.pid * u32::from(t.in_run));
    if pid == 0 {
        return false;
    }
    kill_group(pid, libc::SIGTERM);
    true
}
