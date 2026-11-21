//! An attachment's file name.

/// A safe file name: the last path part, only letters, digits, `.`, `-`, `_`; no leading dots.
pub(super) fn sanitize(name: &str) -> String {
    let s: String = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_start_matches('.')
        .chars()
        .take(120)
        .collect();
    Some(s)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "file".into())
}
