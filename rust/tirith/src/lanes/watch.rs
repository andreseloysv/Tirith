//! Watching a lane whose agent runs: its run ended in the log, it runs long, it went silent, or its process is gone.
use super::ended::ended;
use super::logscan::{quiet_s, run_completed};
use super::long::report_long_running;
use super::send_back::send_back;
use super::session::codex_session;
use crate::app::{alive, kill_group, now_s, tail, Lane, Shared};
use crate::factory::Factory;
use std::collections::HashMap;
use std::path::Path;

/// Seconds since the file last changed (`u64::MAX` when it cannot be read).
fn age_s(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| t.elapsed().map(|d| d.as_secs()).unwrap_or(0))
        .unwrap_or(u64::MAX)
}
/// Its run is over (it committed and said so) but its process lingers: end it like a process exit, so it goes to the merge queue instead of sitting `running` until the idle stop calls it stuck.
async fn end_completed(app: &Shared, l: &Lane) {
    app.event(&l.id, &l.project, "ended", "", "its log says the run completed (run.terminal.completed) although its process is still alive", "");
    app.update_lane(&l.id, |x| {
        x.note = if x.note.is_empty() {
            "run completed in its log (run.terminal.completed); its process was still alive".into()
        } else {
            std::mem::take(&mut x.note)
        }
    });
    kill_group(l.pid.unwrap_or(0), libc::SIGKILL);
    if !app.children.lock().unwrap().contains(&l.id) {
        ended(app, &l.id, None).await; // no waiter task (daemon restart): the killed process ends here, else its waiter ends it
    }
}
/// No new output for `limits.inactivity_minutes`: the agent is stopped and the lane goes back to it.
fn stop_idle(app: &Shared, l: &Lane, f: &Factory) {
    kill_group(l.pid.unwrap_or(0), libc::SIGKILL);
    let why = format!(
        "no new output for {} min (an agent stuck retrying its API call counts as silent)",
        f.limits.inactivity_minutes
    );
    app.event(
        &l.id,
        &l.project,
        "idle",
        "",
        &format!("stopped: {why}"),
        "",
    );
    app.update_lane(&l.id, |x| x.pid = None);
    send_back(
        app,
        l,
        &format!("Your agent was stopped: {why}. Continue the work on your branch."),
    );
}
/// One pass over a lane with a process.
pub(super) async fn watch_lane(
    app: &Shared,
    l: &Lane,
    seen: &mut HashMap<String, (String, u64)>,
    f: &Factory,
) {
    let path = app.home.join(format!("logs/lanes/{}.log", l.id));
    let text = tail(&path, 200);
    let _ = (l.agent == "codex").then(|| codex_session(app, &l.id, &text));
    let log_age = age_s(&path); // seconds since its log last changed
    let quiet = quiet_s(seen, &l.id, &text, log_age, now_s());
    if l.status == "running" && run_completed(&text) {
        return end_completed(app, l).await;
    }
    report_long_running(app, l, &text, f);
    if l.status == "running"
        && now_s().saturating_sub(l.last_activity).min(quiet) > f.limits.inactivity_minutes * 60
    {
        stop_idle(app, l, f);
    } else if !app.children.lock().unwrap().contains(&l.id) && !l.pid.is_some_and(alive) {
        ended(app, &l.id, None).await; // its process ended while the daemon was down
    }
}
