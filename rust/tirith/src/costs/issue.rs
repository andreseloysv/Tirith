//! The issue key an item's text starts with.

/// The key a message names: the one it starts with, else the first one in brackets, as `[ABC-123]` (a bare one inside a text would take `UTF-8` for a key).
pub fn found(text: &str) -> String {
    let lead = issue_key(text);
    let bracketed = text.match_indices('[').map(|(i, _)| issue_key(&text[i..]));
    let inside = |k: &String| text.contains(&format!("[{k}]"));
    Some(lead)
        .filter(|k| !k.is_empty())
        .or_else(|| bracketed.filter(|k| !k.is_empty()).find(inside))
        .unwrap_or_default()
}

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
