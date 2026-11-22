//! Routines run: one now, or every one due on the scheduler's tick.
use super::list::all;
use super::schedule::{due, late, owed};
use super::store::find;
use crate::app::Shared;
use crate::server::s;
use chrono::{DateTime, Local};
/// Runs a routine (`trigger` is `schedule` or `run-now`), `late` when it is not on time: the conversation shows it, and the bot is handed the task as a turn of its own, after what runs before it.
pub(super) fn fire(
    app: &Shared,
    id: &str,
    rid: &str,
    trigger: &str,
    late: &str,
) -> Result<(), String> {
    fire_at(app, (id, rid), trigger, late, Local::now())
}
fn fire_at(
    app: &Shared,
    (id, rid): (&str, &str),
    trigger: &str,
    late: &str,
    now: DateTime<Local>,
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
    let timed = trigger == "schedule";
    let at = crate::expected::instant(now);
    let due = if timed {
        // the occurrence it runs for: not always today's (`catch_up`)
        crate::expected::instant(owed(r, now).ok_or("it is not due")?)
    } else {
        at.clone()
    };
    let stamp = format!("routine:{id}:{rid}");
    if timed && crate::expected::known(app, &key, rid, &due) {
        app.set_kv(&stamp, Some(&at)); // that occurrence is written down: never twice
        return Ok(());
    }
    let told = super::told::told(bot, &title, &task, late);
    let new = crate::expected::New {
        conv: &key,
        routine: rid,
        title: &title,
        trigger,
        kind: "schedule",
        due: &due,
        task: &told,
    };
    crate::expected::expect(app, &new); // written down before anything starts
    app.set_kv(&stamp, Some(&at));
    let noted = if timed && !late.is_empty() {
        crate::expected::late(app, &due, now)
    } else {
        String::new()
    };
    app.chat(
        "routine",
        &format!("Running routine · {title}{noted}"),
        &key,
    );
    if !crate::manager::run_routine(app, &key, "schedule", &told, (trigger, rid)) {
        crate::expected::unhanded(app, &key, rid); // an experiment on it makes the run its A or its B when it starts
    }
    Ok(())
}
/// Writes down what was missed, fires the routines due now, and decides the experiments due; the lanes scheduler calls it on its tick (every ~30 s), so the bots need no timer of their own.
pub fn tick(app: &Shared) {
    tick_at(app, Local::now());
}
/// The tick, as of `now`.
pub(crate) fn tick_at(app: &Shared, now: DateTime<Local>) {
    crate::expected::tick(app, now);
    crate::lab::tick(app);
    for b in all(app) {
        let (id, routines) = (s(&b, "id"), b["routines"].as_array().into_iter().flatten());
        for r in routines.filter(|r| due(r, now)) {
            let (rid, told) = (s(r, "id"), late(r, now));
            drop(fire_at(app, (&id, &rid), "schedule", &told, now));
        }
    }
}
