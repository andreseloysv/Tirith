//! Whether a routine's turn did what it was started for, as far as code is certain: its agent ended well, no «✗», no tool's error proves it could not, and a «∅» says no "could not" on its line and comes from having looked.
use super::admits::admits;
use super::claims::{after, could_not, line, FAILED, NOTHING};
use super::found::oldest;
use super::store::RUNNING;
use super::{reasons::failure, words::short};
use crate::app::{redact_secrets, Shared};
use crate::turns::{Looks, Turn};

/// Why turn `t` (`ok` as its agent ended, its `answer`) is a failure of the run it was; none when it is good, or no run.
pub fn verdict(app: &Shared, t: &Turn, ok: bool, answer: &str) -> Option<String> {
    (!t.routine.is_empty() && t.role == t.conv).then_some(())?;
    let run = oldest(app, &t.conv, &t.routine, RUNNING)?;
    if !run.why.is_empty() {
        return Some(run.why); // Tirith stopped it
    }
    judged(run.kind == "webhook", ok, answer, (t.steps, &t.looks))
}

/// Text as a reason shows it: no secret, one line.
fn shown(text: &str, most: usize) -> String {
    short(&redact_secrets(text), most)
}

/// The rule itself: only what is certain without reading the words' meaning (the run's judge reads that, `admitted.rs`). An event from outside is handed its data: nothing needs to be looked at for a «∅» on it.
pub(super) fn judged(
    handed_data: bool,
    ok: bool,
    answer: &str,
    (steps, looks): (i64, &Looks),
) -> Option<String> {
    if !ok {
        let said = if answer.trim().is_empty() {
            looks.error.as_str()
        } else {
            answer
        };
        return Some(failure(said));
    }
    if answer.trim().is_empty() {
        return Some("su agente terminó sin responder".into());
    }
    if let Some(rest) = after(answer, FAILED) {
        return Some(format!("no pudo: {}", shown(rest, 160)));
    }
    let proof = looks.proof();
    let error = || {
        proof
            .as_ref()
            .map(|p| format!("no pudo: {}", shown(p.line, 160)))
    };
    if proof.as_ref().is_some_and(|p| p.hard) {
        return error(); // the tool's own error is proof enough, whatever the words
    }
    let Some(rest) = after(answer, NOTHING) else {
        // words are for the judge to read: "I can't see why it fired twice" beside the result is no failure, and what is down may be the news
        return None;
    };
    let claim = line(rest);
    if admits(claim) || could_not(claim) {
        error().or_else(|| Some(format!("dijo ∅, pero no pudo: {}", shown(rest, 160))))
    } else if let Some(look) = looks.blind() {
        error().or_else(|| Some(format!("dijo ∅, pero no pudo mirar ({})", shown(look, 80))))
    } else if proof.is_some() {
        error() // ∅ over an error that stands is no silence
    } else if steps == 0 && !handed_data {
        Some("dijo ∅ sin mirar: no usó ninguna herramienta".into())
    } else {
        None
    }
}
