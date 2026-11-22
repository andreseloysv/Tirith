//! Routines run: one now, or every one due on the scheduler's tick.
use super::list::all;
use super::schedule::due;
use super::store::find;
use crate::app::{now_iso, Shared};
use crate::server::s;
use chrono::Local;
/// Runs a routine (`trigger` is `schedule` or `run-now`): the conversation shows it, and the bot is handed the task as a turn of its own, after what runs before it.
pub(super) fn fire(app: &Shared, id: &str, rid: &str, trigger: &str) -> Result<(), String> {
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
    let (now, timed) = (Local::now(), trigger == "schedule");
    let due = timed
        .then(|| crate::expected::today(&s(r, "at"), now))
        .flatten();
    let due = due.unwrap_or_else(now_iso);
    let stamp = format!("routine:{id}:{rid}");
    if timed && crate::expected::known(app, &key, rid, &due) {
        app.set_kv(&stamp, Some(&now_iso())); // that occurrence is written down: never twice
        return Ok(());
    }
    let role_is_turn = bot["session"] == "turn";
    let its = if role_is_turn {
        " It is a task, not a review."
    } else {
        ""
    };
    let told = format!("Routine «{title}» is due now.{its} Do it and report the result here in a few lines ({}); ask nothing: {task}", super::REPORT);
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
    app.set_kv(&stamp, Some(&now_iso()));
    let late = if timed {
        crate::expected::late(app, &due, now)
    } else {
        String::new()
    };
    app.chat("routine", &format!("Running routine · {title}{late}"), &key);
    if !crate::manager::run_routine(app, &key, "schedule", &told, (trigger, rid)) {
        crate::expected::unhanded(app, &key, rid); // an experiment on it makes the run its A or its B when it starts
    }
    Ok(())
}
/// Writes down what was missed, fires the routines due now, and decides the experiments due; the lanes scheduler calls it on its tick (every ~30 s), so the bots need no timer of their own.
pub fn tick(app: &Shared) {
    crate::expected::tick(app, Local::now());
    crate::lab::tick(app);
    all(app).iter().for_each(|b| {
        b["routines"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| due(r, Local::now()))
            .for_each(|r| drop(fire(app, &s(b, "id"), &s(r, "id"), "schedule")))
    });
}
