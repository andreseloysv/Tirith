//! Times, days and texts as the owner reads them in one line.
use super::calendar::{local, next};
use crate::server::s;
use chrono::{DateTime, Datelike, Local};
use serde_json::Value;

const DAYS: [&str; 7] = ["lun", "mar", "mié", "jue", "vie", "sáb", "dom"];

/// `text` on one line, cut at `most` characters.
pub(super) fn short(text: &str, most: usize) -> String {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() <= most {
        return line;
    }
    let cut: String = line.chars().take(most.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

/// «lun 23/11 08:00».
pub(super) fn at(t: DateTime<Local>) -> String {
    let day = DAYS[t.weekday().num_days_from_monday() as usize];
    format!("{day} {}", t.format("%d/%m %H:%M"))
}

/// A kept instant, so.
pub(super) fn when(kept: &str) -> String {
    local(kept).map_or_else(|| kept.to_string(), at)
}

/// «cada día», «lun a vie», «lun, mié».
fn days_of(days: &[u64]) -> String {
    let mut d: Vec<u64> = days
        .iter()
        .copied()
        .filter(|d| (1..=7).contains(d))
        .collect();
    d.sort_unstable();
    d.dedup();
    let name = |d: &u64| DAYS[(*d - 1) as usize];
    match d.as_slice() {
        [1, 2, 3, 4, 5, 6, 7] => "cada día".into(),
        [a, .., b] if d.len() > 2 && b - a + 1 == d.len() as u64 => {
            format!("{} a {}", name(a), name(b))
        }
        _ => d.iter().map(name).collect::<Vec<_>>().join(", "),
    }
}

fn days_in(r: &Value) -> Vec<u64> {
    let days = r["days"].as_array().into_iter().flatten();
    days.filter_map(Value::as_u64).collect()
}

/// « · lun a vie 08:00».
pub(super) fn days_and_time(r: &Value) -> String {
    format!(" · {} {}", days_of(&days_in(r)), s(r, "at"))
}

/// A timed routine's days and time, and when it next runs (`label`).
pub(super) fn runs(r: &Value, now: DateTime<Local>, label: &str) -> String {
    let Some(next) = next((&s(r, "at"), &days_in(r)), now) else {
        return String::new();
    };
    format!("{} · {label}: {}", days_and_time(r), at(next))
}

/// What the line of a new timed routine adds: its days, its time and when it first runs, so nobody waits for it on a day it has none.
pub fn first(r: &Value, now: DateTime<Local>) -> String {
    if s(r, "trigger") != "schedule" {
        return String::new();
    }
    runs(r, now, "primera vez")
}

/// What a failed run's reason reads: `cause` after "no pudo: ", unless it already begins with a could-not (never "no pudo: no pudo …"). `lead` goes first.
pub(super) fn not_able(lead: &str, cause: &str) -> String {
    const STARTS: [&str; 11] = [
        "no pudo",
        "no se pudo",
        "no pude",
        "no puedo",
        "no he podido",
        "no ha podido",
        "could not",
        "couldn't",
        "couldn’t",
        "unable to",
        "cannot",
    ];
    let low = cause
        .trim_start_matches(['«', '"', '`', '*'])
        .to_lowercase();
    if STARTS.iter().any(|s| low.starts_with(s)) {
        format!("{lead}{cause}")
    } else {
        format!("{lead}no pudo: {cause}")
    }
}
