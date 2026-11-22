//! The message a conversation is handed: the memory, the reports waiting for it and the words of the owner, a room, an event or a routine, with what starts its turn.
use super::talk::{Job, Msg, Talk};
use crate::turns::By;

/// What the message says apart from the memory: the reports waiting (and how many were cut), then the routine's task (`job`) or `text`. Empty when there is nothing to say.
fn asked(t: &mut Talk, text: &str, job: Option<&Job>) -> Vec<String> {
    let left = std::mem::take(&mut t.left_out);
    let cut = (left > 0).then(|| {
        format!("{left} older reports were left out: the conversation was busy for too long")
    });
    let notes = cut.into_iter().chain(std::mem::take(&mut t.notes));
    let words = job.map_or(text, |j| j.text.as_str());
    notes
        .map(|n| format!("[tirith] {n}"))
        .chain((!words.is_empty()).then(|| words.to_string()))
        .collect()
}

/// The message for `project`: its text with the `memory` in front, and what starts its turn: the routine's run (`job`), else `by` when given, else the trigger that waited for it, taken now so it never outlives this message. None when there is nothing to say.
pub(super) fn compose(
    t: &mut Talk,
    project: &str,
    (memory, text): (Option<String>, &str),
    (job, by): (Option<&Job>, Option<By>),
) -> Option<Msg> {
    let asked = asked(t, text, job);
    (!asked.is_empty()).then_some(())?;
    let waiting = crate::turns::take_trigger(project);
    crate::turns::told(project, &asked.join("\n\n")); // what started the turn, for its capture
    let all: Vec<String> = memory.into_iter().chain(asked).collect();
    let by = match (job, by) {
        (Some(j), _) => j.by.clone(),
        (_, Some(by)) => by,
        _ => waiting,
    };
    Some(Msg {
        text: all.join("\n\n"),
        by,
        job: job.is_some(),
    })
}
