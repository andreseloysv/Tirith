//! Plan items the autopilot leaves alone: role items, items waiting for the owner, and items waiting for another open item.

/// Role items (the factory's `[plan] role_prefixes`) are attended by a role, and items marked as waiting for the owner (`[plan] owner_wait`) wait: the autopilot starts no lane for either.
pub fn is_autopilot_skipped(text: &str) -> bool {
    let t = text.trim_start().to_lowercase();
    let f = crate::factory::current();
    f.words("plan", "role_prefixes")
        .iter()
        .any(|p| t.starts_with(&p.to_lowercase()))
        || f.words("plan", "owner_wait")
            .iter()
            .any(|w| t.contains(&w.to_lowercase()))
}
/// The words of five letters or more, lowercased.
fn long_words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 5)
        .map(String::from)
        .collect()
}
/// Whether `t` (lowercased) quotes the open item `o` or shares most of its words.
fn names_item(t: &str, o: &str) -> bool {
    let (ol, ow) = (o.trim().to_lowercase(), long_words(o));
    ol != t.trim()
        && !ol.is_empty()
        && (t.contains(&ol)
            || (ow.len() >= 3
                && ow.iter().filter(|w| t.contains(w.as_str())).count() * 10 >= ow.len() * 6))
}
/// Items that wait for another open item (the factory's `[plan] waits_for` words) by quoting it or sharing most of its words.
pub(super) fn waits_for_open_item(text: &str, open: &[&str]) -> bool {
    let t = text.to_lowercase();
    crate::factory::current()
        .words("plan", "waits_for")
        .iter()
        .any(|m| t.contains(&m.to_lowercase()))
        && open.iter().any(|o| names_item(&t, o))
}
