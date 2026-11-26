//! The issue key an item's text starts with.

/// `ABC-123` for "ABC-123: do it", "[ABC-123] do it" or "ABC-123 do it"; empty when the text starts with none.
pub fn issue_key(text: &str) -> String {
    let t = text.trim_start().trim_start_matches('[');
    let letters = t
        .find(|c: char| !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'))
        .unwrap_or(t.len());
    let (name, rest) = t.split_at(letters);
    let number: String = rest
        .strip_prefix('-')
        .unwrap_or("")
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let after = rest.get(1 + number.len()..).and_then(|r| r.chars().next());
    let named = name.starts_with(|c: char| c.is_ascii_uppercase()) && name.len() >= 2;
    let ends = after.is_none_or(|c| !c.is_ascii_alphanumeric());
    if named && !number.is_empty() && ends {
        format!("{name}-{number}")
    } else {
        String::new()
    }
}
