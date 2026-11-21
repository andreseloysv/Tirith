//! Routines run: one now, or every one due on the scheduler's tick.
use super::list::all;
use super::schedule::due;
use super::store::find;
use crate::app::{now_iso, Shared};
use crate::server::s;
use chrono::Local;
/// Runs a routine now: the conversation shows it, and the bot is handed the task.
pub(super) fn fire(app: &Shared, id: &str, rid: &str) -> Result<(), String> {
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
    crate::turns::trigger(&key, "schedule", rid); // the page's Run now said run-now first
    crate::manager::emit_event(app, &key, "schedule", &format!("Routine «{title}» is due now. Do it and report the result here in a few lines (when nothing is new, one line that starts with ∅); ask nothing: {task}"), None);
    Ok(())
}
/// Fires the routines due now; the lanes scheduler calls it on its tick (every ~30 s), so the bots need no timer of their own.
pub fn tick(app: &Shared) {
    all(app).iter().for_each(|b| {
        b["routines"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| due(r, Local::now()))
            .for_each(|r| drop(fire(app, &s(b, "id"), &s(r, "id"))))
    });
}
