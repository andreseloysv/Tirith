//! A lane asks the manager and waits; the answer reaches it live or in a resumed run.
use super::live::tell;
use super::stop::kill_agent;
use crate::app::{alive, Lane, Shared};
use crate::manager;

/// `neobot ask` from a lane: it waits, and the manager is told how to answer.
pub fn ask(app: &Shared, id: &str, question: &str) -> Result<Lane, String> {
    let lane = app.lane(id).ok_or(format!("no lane '{id}'"))?;
    (lane.status == "running" && !question.trim().is_empty())
        .then_some(())
        .ok_or_else(|| {
            format!(
                "lane {id} is {}: only a working lane asks, with a question",
                lane.status
            )
        })?;
    // A question about money stops the agent: it must not spend before the answer (it resumes with it).
    let updated = app.update_lane(id, |l| {
        (l.status, l.question, l.resume) =
            ("waiting".into(), question.trim().into(), String::new());
        is_payment(question).then(|| kill_agent(l.pid.take()));
    });
    let (q, owner) = (
        question.trim(),
        format!(
            "neobot ask-owner --lane {id} --kind <kind> \"<the question, in the owner's words>\""
        ),
    );
    let told = format!("lane {id} ({}: {}) asks: {q}\nAnswer with: neobot answer {id} \"<answer>\". When only the owner can answer, ask them with {owner}: their answer goes straight to the lane.", lane.project, lane.item);
    manager::emit_event(
        app,
        &lane.project,
        "lane.asked",
        &format!("lane {id} asks the manager: {q}"),
        Some(&told),
    );
    updated.ok_or("lane vanished".into())
}
/// `neobot answer` from the manager, or the owner's answer (`who`): a live message when the agent still runs, else a resumed run.
pub fn answer(app: &Shared, id: &str, text: &str, who: &str) -> Result<Lane, String> {
    let lane = app.lane(id).ok_or(format!("no lane '{id}'"))?;
    (lane.status == "waiting")
        .then_some(())
        .ok_or_else(|| format!("lane {id} is {}, not waiting for an answer", lane.status))?;
    let live = lane.pid.is_some_and(alive);
    let _ = live.then(|| tell(id, &format!("Answer from {who}: {text}")));
    let lane = app.update_lane(id, |l| {
        l.resume = if l.session.is_empty() {
            format!(
                "You asked the manager: {}\nAnswer from {who}: {text}",
                l.question
            )
        } else {
            format!("Answer from {who}: {text}")
        };
        let _ = (!live).then(|| l.status = "queued".into());
    });
    app.sched_wake.notify_one();
    lane.ok_or("lane vanished".into())
}
pub(super) fn is_payment(question: &str) -> bool {
    let q = question.to_lowercase();
    [
        "$", "€", "usd", "eur", "pay", "paid", "spend", "cost", "price", "charge", "billing",
        "pago", "pagar", "gasto", "gastar", "coste", "precio", "cuesta",
    ]
    .iter()
    .any(|w| q.contains(w))
}
