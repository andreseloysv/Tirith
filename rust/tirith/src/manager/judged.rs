//! Reading what a judge answered, and which card an event's line names.
/// The card an event's line names (`Card o12 …`, `The owner answered o12 …`): its first word shaped like one.
pub fn card_id_of(shown: &str) -> Option<String> {
    shown
        .split(|c: char| !c.is_ascii_alphanumeric())
        .find(|w| w.len() > 1 && w.starts_with('o') && w[1..].bytes().all(|b| b.is_ascii_digit()))
        .map(String::from)
}
/// A judge's answer read as (label, score, why): its first word, a number when the rubric asks for a score from 0 to 1, and the rest of its first line.
pub(super) fn judged(text: &str) -> (String, Option<f64>, String) {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or_default();
    let head = line
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '-')
        .to_lowercase();
    let why = line[line
        .to_lowercase()
        .find(&head)
        .map(|i| i + head.len())
        .unwrap_or(0)..]
        .trim_start_matches(|c: char| c.is_whitespace() || "—–-|:,.;".contains(c))
        .to_string();
    match head.parse::<f64>() {
        Ok(n) if (0.0..=1.0).contains(&n) => (String::new(), Some(n), why),
        _ => (head, None, why),
    }
}
