//! The runs of routines waiting for their conversation: each is a turn of its own, handed over alone, in the order they were due, when the conversation is free.
use super::deliver::deliver;
use super::talk::{talk, Job, Msg};
use super::waiting::held;
use crate::app::Shared;

/// A run of a routine is due for `project`: it waits its turn, and starts at once when nothing else runs.
pub(super) fn queue_job(app: &Shared, project: &str, job: Job) {
    talk(project, |t| t.jobs.push_back(job));
    deliver(app, project, "", false);
}

/// The next run waiting, when the conversation is free (no turn, no message on its way, no report waiting): taken from the queue, and the conversation is its own from this moment.
pub(super) fn reserve(project: &str) -> Option<Job> {
    talk(project, |t| {
        let free = !t.busy && !held(t) && t.pending.is_empty() && !t.notes_due;
        let free = free && crate::turns::idle(project);
        let job = free.then(|| t.jobs.pop_front()).flatten();
        t.taken |= job.is_some();
        job
    })
}

/// A routine's run starts: it is A or B of the experiment on it, and its own effort or model applies, each ending its session first when its turn needs a fresh one.
pub(super) fn start_run(app: &Shared, project: &str, job: &Job) {
    crate::expected::started(app, project, &job.by.1);
    crate::lab::fired(app, project, &job.by.1, &job.text);
    crate::bots::tune(app, project, &job.by.1);
}

/// What a run held (its seat in an experiment, its tuning) is freed: it ended without a turn row, failed or never started.
pub fn release_run(app: &Shared, project: &str) {
    crate::lab::released(app, project);
    crate::bots::untune(app, project);
}

/// The messages an exec agent takes for one turn: a routine's run alone, else the leading ones that are not.
pub(super) fn take_group(pending: &mut Vec<Msg>) -> Vec<Msg> {
    let n = match pending.first() {
        Some(m) if m.job => 1,
        _ => pending.iter().take_while(|m| !m.job).count(),
    };
    pending.drain(..n).collect()
}
