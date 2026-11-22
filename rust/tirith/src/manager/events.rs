//! Events and their routes: who hears an event, as a chat line, a turn now or a note with the next turn.
use super::deliver::report;
use super::evals::judge_evals;
use super::jobs::queue_job;
use super::queue::queue;
use super::talk::Job;
use crate::app::Shared;
use crate::factory::Factory;
use crate::turns::By;
use std::collections::BTreeSet;

/// The one way code emits an event: its row is kept and shown on the page, and every `[[routes]]` on it in the factory decides who hears it — a `chat` line (on the phones too with `phone`), and for its role a `turn` now or a `note` with its next turn. `told` is what the role is handed when it differs from the line shown. A turn's answer reaches the phones only when an event that started it had `phone` (or the owner wrote).
pub fn emit_event(app: &Shared, project: &str, on: &str, shown: &str, told: Option<&str>) {
    emitted(app, project, on, shown, told);
}
/// `emit_event`, with the event's id back (a turn's row keeps it).
pub fn emitted(app: &Shared, project: &str, on: &str, shown: &str, told: Option<&str>) -> i64 {
    emit_by(app, project, (on, shown, told), None)
}
/// A routine is due (`on` is `schedule` or `webhook`, `task` what its bot is handed): its run is a turn of its own, begun by `by` (the trigger and the routine's id), after the runs and messages before it in that conversation.
pub fn run_routine(app: &Shared, project: &str, on: &str, task: &str, by: (&str, &str)) {
    emit_by(
        app,
        project,
        (on, task, None),
        Some((by.0.into(), by.1.into())),
    );
}
fn emit_by(
    app: &Shared,
    project: &str,
    (on, shown, told): (&str, &str, Option<&str>),
    job: Option<By>,
) -> i64 {
    let event = app.event("", project, on, "", shown, "");
    let routes = app.cfg().factory.routes_on(on);
    let here_p = app.conv(project);
    if routes.iter().any(|r| r.2 && r.3) {
        report(app, here_p, shown);
    } else if routes.iter().any(|r| r.2) {
        app.chat("tirith", shown, here_p);
    }
    let mut heard: BTreeSet<String> = BTreeSet::new();
    for (to, mode, _, phone, after) in routes {
        // A turn role heard as a turn runs its own pass now (the metrics run it, like the CEO's).
        let _ = (mode == "turn" && after.is_none())
            .then(|| crate::metrics::heard(app, &to, on, told.unwrap_or(shown)));
        let Some(dest) = dest_of(&app.cfg().factory, &to, on, here_p) else {
            continue;
        };
        if !heard.insert(dest.clone()) {
            continue;
        }
        let text = told.unwrap_or(shown);
        match job.clone().filter(|_| mode == "turn") {
            Some(by) => queue_job(
                app,
                &dest,
                Job {
                    text: text.into(),
                    by,
                    phones: phone,
                },
            ),
            None => {
                crate::turns::on_event(&dest, on, &mode);
                queue(app, &dest, text, &mode, phone);
            }
        }
    }
    judge_evals(app, project, on, shown, event);
    event
}
/// The conversation a route to role `to` hands its event to, if any.
fn dest_of(f: &Factory, to: &str, on: &str, here_p: &str) -> Option<String> {
    // Only chat roles are handed text here, in the conversation of their scope; a turn role is run by whoever watches its condition (the metrics run the CEO through `run_turn`, like any turn role).
    (f.role::<String>(to, "session").as_deref() == Some("chat")).then_some(())?;
    let own = f.role::<String>(to, "scope").as_deref() == Some("own");
    // A due routine (its bot's own `schedule` or `webhook` route) is for that bot's conversation alone, and a conversation that more than one route reaches (the factory's schedule route and the bot's) hears the event once.
    (!(matches!(on, "schedule" | "webhook") && own && to != here_p)).then_some(())?;
    Some(if own { to } else { here_p }.to_string())
}
