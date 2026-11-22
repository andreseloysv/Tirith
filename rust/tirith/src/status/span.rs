//! A length of time in a few characters.
/// `seconds` in its two biggest units.
pub(super) fn span(seconds: u64) -> String {
    let (days, hours, minutes) = (seconds / 86400, seconds / 3600 % 24, seconds / 60 % 60);
    match (days, hours, minutes) {
        (0, 0, 0) => "less than 1 min".into(),
        (0, 0, m) => format!("{m} min"),
        (0, h, m) => format!("{h} h {m} min"),
        (d, h, _) => format!("{d} d {h} h"),
    }
}
