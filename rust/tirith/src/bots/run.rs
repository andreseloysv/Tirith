//! Routines run: one now, or every one due on the scheduler's tick.
use super::list::all;
use super::schedule::{due, late};
use super::store::find;
use crate::app::{now_iso, Shared};
use crate::server::s;
use chrono::Local;
/// Runs a routine (`trigger` is `schedule` or `run-now`), `late` when it is not on time: the conversation shows it, and the bot is handed the task as a turn of its own, after what runs before it.
pub(super) fn fire(
    app: &Shared,
    id: &str,
    rid: &str,
    trigger: &str,
    late: &str,
) -> Result<(), String> {
    let mut bots = all(app);
    let bot = find(&mut bots, id)?;
    let (key, r) = (
        s(bot, "key"),
        bot["routines"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|r| r["id"] == rid)
            .ok_or("no such routine")?,
    );
    let (title, task) = (s(r, "title"), s(r, "prompt"));
    app.set_kv(&format!("routine:{id}:{rid}"), Some(&now_iso()));
    app.chat("routine", &format!("Running routine · {title}"), &key);
    let told = format!("Routine «{title}» is due now{late}. Do it and report the result here in a few lines (when nothing is new, one line that starts with ∅); ask nothing: {task}");
    crate::manager::run_routine(app, &key, "schedule", &told, (trigger, rid)); // an experiment on it makes the run its A or its B when it starts
    Ok(())
}
/// Fires the routines due now, and decides the experiments due; the lanes scheduler calls it on its tick (every ~30 s), so the bots need no timer of their own.
pub fn tick(app: &Shared) {
    crate::lab::tick(app);
    let now = Local::now();
    all(app).iter().for_each(|b| {
        b["routines"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| due(r, now))
            .for_each(|r| {
                drop(fire(
                    app,
                    &s(b, "id"),
                    &s(r, "id"),
                    "schedule",
                    &late(r, now),
                ))
            })
    });
}
