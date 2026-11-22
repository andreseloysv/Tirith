//! Whether a routine's turn did what it was started for: its agent ended well, it did not say it could not (in any words, with or without a mark), no tool's error proves it could not, and a «∅» comes from having looked.
use super::admits::{admits, repeats, trouble};
use super::claims::{after, could_not, FAILED, NOTHING};
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

/// The rule itself. An event from outside is handed its data: nothing needs to be looked at for a «∅» on it.
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
        return Some(format!("no pudo: {}", short(rest, 160)));
    }
    let proof = looks.proof();
    let error = || {
        proof
            .as_ref()
            .map(|p| format!("no pudo: {}", short(&redact_secrets(p.line), 160)))
    };
    if proof.as_ref().is_some_and(|p| p.hard) {
        return error(); // the tool's own error is proof enough, whatever the words
    }
    let Some(rest) = after(answer, NOTHING) else {
        let worded = proof
            .as_ref()
            .is_some_and(|p| trouble(answer) || repeats(answer, p.line));
        let nothing = !handed_data && looks.nothing_worked();
        if worded || nothing || admits(answer) {
            return error().or_else(|| Some(format!("no pudo: {}", short(answer, 160))));
        }
        return None;
    };
    if admits(answer) || could_not(answer) {
        error().or_else(|| Some(format!("dijo ∅, pero no pudo: {}", short(rest, 160))))
    } else if let Some(look) = looks.blind() {
        error().or_else(|| Some(format!("dijo ∅, pero no pudo mirar ({})", short(look, 80))))
    } else if proof.is_some() {
        error() // ∅ over an error that stands is no silence
    } else if steps == 0 && !handed_data {
        Some("dijo ∅ sin mirar: no usó ninguna herramienta".into())
    } else {
        None
    }
}
