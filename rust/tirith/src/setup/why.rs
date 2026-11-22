//! Why a connection check failed, in the agent's own words: what it wrote on stderr during that turn.
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// The lines that matter most in an error.
const TELLING: [&str; 8] = [
    "error",
    "denied",
    "permission",
    "unauthorized",
    "expired",
    "quota",
    "not logged in",
    "credential",
];

/// The telling lines added to `log` after byte `from`, the last three; with none, its last line.
pub(super) fn since(log: &Path, from: u64) -> String {
    let mut text = String::new();
    let Ok(mut file) = std::fs::File::open(log) else {
        return text;
    };
    if file.seek(SeekFrom::Start(from)).is_err() || file.read_to_string(&mut text).is_err() {
        return String::new();
    }
    picked(&text)
}

pub(super) fn picked(text: &str) -> String {
    let lines: Vec<&str> = text
        .lines()
        .map(|l| l.trim().trim_start_matches(['•', ' ']))
        .filter(|l| !l.is_empty())
        .collect();
    let telling: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| {
            let low = l.to_lowercase();
            TELLING.iter().any(|word| low.contains(word))
        })
        .collect();
    match telling.is_empty() {
        true => lines.last().map(|l| l.to_string()).unwrap_or_default(),
        false => telling[telling.len().saturating_sub(3)..].join(" | "),
    }
}
