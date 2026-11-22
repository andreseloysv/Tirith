//! A value as it comes on stdin: typed unseen on a terminal, or piped whole (a PEM has lines) less its last line end.
use super::names;
use std::io::{IsTerminal, Read};

pub(super) fn read(name: &str) -> Result<String, String> {
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        return crate::setup::read_secret(&format!("Value of {name}"));
    }
    piped(stdin.lock())
}

pub(super) fn piped(from: impl Read) -> Result<String, String> {
    let mut bytes = vec![];
    from.take(names::MOST as u64 + 3) // a line end, and one byte that says it is too long
        .read_to_end(&mut bytes)
        .map_err(|e| format!("stdin: {e}"))?;
    let text = String::from_utf8(bytes).map_err(|_| "stdin holds no UTF-8 text")?;
    let value = match text.strip_suffix('\n') {
        Some(line) => line.strip_suffix('\r').unwrap_or(line),
        None => &text,
    };
    names::value(value)?;
    Ok(value.to_string())
}
