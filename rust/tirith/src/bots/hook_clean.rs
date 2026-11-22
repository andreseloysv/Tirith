//! What a webhook event shows outside the data block: a label made only of name characters, and a title as one quoted line, so nothing a sender wrote reads as a sentence addressed to a reader.
/// The longest label or title shown.
const SHOWN: usize = 80;
/// An event's label as the owner's `when` names it: letters, digits and `: . _ * -` only (anything else is `_`), 80 characters at most.
pub(super) fn ident(label: &str) -> String {
    label
        .chars()
        .map(|c| match c {
            c if c.is_ascii_alphanumeric() => c,
            ':' | '.' | '_' | '*' | '-' => c,
            _ => '_',
        })
        .take(SHOWN)
        .collect()
}
/// A title for the chat or the event line: one line, controls gone, 80 characters at most, quoted in «».
pub(super) fn quoted(title: &str) -> String {
    let one: String = title
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(SHOWN)
        .collect();
    format!("«{}»", one.trim())
}
/// A value from the event inside a task, between two marks no sender can guess, said to be data: it is never read again for `{title}` and the like.
pub(super) fn fenced(mark: &str, value: &str) -> String {
    format!("⟦outside-data {mark}⟧{value}⟦/outside-data {mark}⟧")
}
