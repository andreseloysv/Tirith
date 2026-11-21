//! Events and their routes: who hears an event, as a chat line, a turn now or a note with the next turn.
use super::deliver::{deliver, report};
use super::evals::judge_evals;
use super::talk::talk;
use crate::app::Shared;
use crate::factory::Factory;
use std::collections::BTreeSet;
use std::time::Duration;
/// The one way code emits an event: its row is kept and shown on the page, and every `[[routes]]` on it in the factory decides who hears it — a `chat` line (on the phones too with `phone`), and for its role a `turn` now or a `note` with its next turn. `told` is what the role is handed when it differs from the line shown. A turn's answer reaches the phones only when an event that started it had `phone` (or the owner wrote).
pub fn emit_event(app: &Shared, project: &str, on: &str, shown: &str, told: Option<&str>) {
    emitted(app, project, on, shown, told);
}
/// `emit_event`, with the event's id back (a turn's row keeps it).
pub fn emitted(app: &Shared, project: &str, on: &str, shown: &str, told: Option<&str>) -> i64 {
    let event = app.event("", project, on, "", shown, "");
    let routes = app.cfg().factory.routes_on(on);
    let here_p = app.conv(project);
    if routes.iter().any(|r| r.2 && r.3) {
        report(app, here_p, shown);
    } else if routes.iter().any(|r| r.2) {
        app.chat("tirith", shown, here_p);
    }
    let mut heard: BTreeSet<String> = BTreeSet::new();
    for (to, mode, _, phone, _) in routes {
        let Some(dest) = dest_of(&app.cfg().factory, &to, on, here_p) else {
            continue;
        };
        let _ = heard
            .insert(dest.clone())
            .then(|| crate::turns::on_event(&dest, on, &mode))
            .map(|_| queue(app, &dest, told.unwrap_or(shown), &mode, phone));
    }
    judge_evals(app, project, on, shown, event);
    event
}
/// The conversation a route to role `to` hands its event to, if any.
fn dest_of(f: &Factory, to: &str, on: &str, here_p: &str) -> Option<String> {
    // Only chat roles are handed text here, in the conversation of their scope; a turn role is run by whoever watches its condition (the metrics run the CEO through `run_turn`, like any turn role).
    (f.role::<String>(to, "session").as_deref() == Some("chat")).then_some(())?;
    let own = f.role::<String>(to, "scope").as_deref() == Some("own");
    // A due routine (its bot's own `schedule` route) is for that bot's conversation alone, and a conversation that more than one route reaches (the factory's schedule route and the bot's) hears the event once.
    (!(on == "schedule" && own && to != here_p)).then_some(())?;
    Some(if own { to } else { here_p }.to_string())
}
/// A routed event for one conversation: with `turn` handed over (gathered with the other events of the next `limits.batch_seconds`, so many events cost one turn) and its `phone` marking that turn's answer; with `note` it waits for the conversation's next message. Anything else hears nothing.
pub(super) fn queue(app: &Shared, project: &str, text: &str, mode: &str, phone: bool) {
    let turn = match mode {
        "turn" => true,
        "note" => false,
        _ => return,
    };
    let first = talk(project, |t| {
        t.notes.push(text.to_string());
        t.notes.drain(..t.notes.len().saturating_sub(30));
        t.phones |= turn && phone;
        turn && !std::mem::replace(&mut t.due, true)
    });
    if first {
        let (app, project, batch) = (
            app.clone(),
            project.to_string(),
            app.cfg().factory.limits.batch_seconds,
        );
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(batch)).await;
            talk(&project, |t| t.due = false);
            deliver(&app, &project, "", false);
        });
    }
}
