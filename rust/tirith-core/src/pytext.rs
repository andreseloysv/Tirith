//! Python text semantics the parsers rely on (`str.splitlines`, `str.strip`, `str.split`, `or`, `str()`),
//! so ported code sees the same lines and words as the Python collector.

use std::path::{Path, PathBuf};
use toml::Value;

/// `str.isspace` for one char: Unicode white space plus the ASCII separators `\x1c`-`\x1f`.
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// `str.strip()`.
pub fn strip(text: &str) -> &str {
    text.trim_matches(is_space)
}

/// `str.split()` with no argument: runs of white space separate the words.
pub fn split_words(text: &str) -> Vec<&str> {
    text.split(is_space).filter(|w| !w.is_empty()).collect()
}

/// `" ".join(text.split())`.
pub fn squash(text: &str) -> String {
    split_words(text).join(" ")
}

/// `str.splitlines()`: `\n`, `\r`, `\r\n`, `\v`, `\f`, `\x1c`-`\x1e`, `\x85`, ` `, ` `.
pub fn splitlines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let is_break = matches!(
            c,
            '\n' | '\r' | '\u{0b}' | '\u{0c}' | '\u{1c}' | '\u{1d}' | '\u{1e}' | '\u{85}' | '\u{2028}' | '\u{2029}'
        );
        if !is_break {
            continue;
        }
        out.push(&text[start..i]);
        start = i + c.len_utf8();
        if c == '\r' {
            if let Some(&(j, '\n')) = chars.peek() {
                chars.next();
                start = j + 1;
            }
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

/// Text as Python's `read_text` returns it: `\r\n` and `\r` become `\n`.
pub fn universal_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Python truthiness of a TOML value (`value or default`).
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::String(s) => !s.is_empty(),
        Value::Integer(i) => *i != 0,
        Value::Float(f) => *f != 0.0,
        Value::Boolean(b) => *b,
        Value::Array(a) => !a.is_empty(),
        Value::Table(t) => !t.is_empty(),
        Value::Datetime(_) => true,
    }
}

/// `str(value)` for a scalar TOML value the way Python prints it.
pub fn str_of(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => format!("{f:?}"),
        Value::Boolean(true) => "True".to_string(),
        Value::Boolean(false) => "False".to_string(),
        Value::Datetime(d) => d.to_string(),
        other => other.to_string(),
    }
}

/// `str(table.get(key) or "")`, `None` when the key is missing or falsy.
pub fn get_str(table: &toml::Table, key: &str) -> Option<String> {
    table.get(key).filter(|v| truthy(v)).map(str_of)
}

/// `Path.expanduser()` for `~` and `~/...`.
pub fn expanduser(path: &str) -> PathBuf {
    let home = || std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
    if path == "~" {
        return home().unwrap_or_else(|| PathBuf::from(path));
    }
    match (path.strip_prefix("~/"), home()) {
        (Some(rest), Some(h)) => h.join(rest),
        _ => PathBuf::from(path),
    }
}

/// A path as `pathlib` spells it: no `.` components, no doubled separators, no trailing separator.
pub fn norm(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for part in path.components() {
        if part != Component::CurDir {
            out.push(part.as_os_str());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitlines_matches_python() {
        assert_eq!(splitlines("a\nb\r\nc\rd"), vec!["a", "b", "c", "d"]);
        assert_eq!(splitlines("a\n"), vec!["a"]);
        assert_eq!(splitlines("a\n\nb"), vec!["a", "", "b"]);
        assert_eq!(splitlines("a\u{2028}b\u{0c}c"), vec!["a", "b", "c"]);
        assert!(splitlines("").is_empty());
    }

    #[test]
    fn strip_and_squash() {
        assert_eq!(strip(" \u{1c}x y\t"), "x y");
        assert_eq!(squash("  a \t b\n c "), "a b c");
    }

    #[test]
    fn norm_drops_current_dir() {
        assert_eq!(norm(Path::new("./a//b/./c")), PathBuf::from("a/b/c"));
    }
}
