//! A bot's numbers by skill: how often it was used, how often its validation passed, and the tokens of its routine's runs before its first use against after.
use super::routines::mean;
use super::rows::Row;
use serde_json::{json, Value};

/// The mean tokens of some runs, rounded.
fn tokens(rows: &[&Row]) -> Option<i64> {
    mean(rows.iter().map(|r| r.tokens as f64)).map(|t| t.round() as i64)
}

/// One skill's numbers: its uses, its validated uses, and its routine's tokens before and after (the routine of its first use).
fn skill(name: &str, rows: &[Row]) -> Value {
    let used: Vec<&Row> = rows
        .iter()
        .filter(|r| r.skills.iter().any(|s| s == name))
        .collect();
    let first = used[0];
    let runs: Vec<&Row> = rows.iter().filter(|r| r.routine == first.routine).collect();
    let before = tokens(&runs.iter().copied().filter(|r| r.id < first.id).collect::<Vec<_>>());
    let after = tokens(&runs.iter().copied().filter(|r| r.id >= first.id).collect::<Vec<_>>());
    let saved = before
        .zip(after)
        .filter(|(b, _)| *b > 0)
        .map(|(b, a)| ((b - a) as f64 * 100.0 / b as f64).round());
    json!({
        "name": name,
        "used": used.len(),
        "validated": used.iter().filter(|r| r.validated).count(),
        "routine": first.routine,
        "tokens_before": before,
        "tokens_after": after,
        "saved_percent": saved,
    })
}

/// Every skill the bot's runs read, in the order it first used them.
pub fn by_skill(rows: &[Row]) -> Vec<Value> {
    let mut names: Vec<&String> = vec![];
    rows.iter().flat_map(|r| &r.skills).for_each(|s| {
        let _ = (!names.contains(&s)).then(|| names.push(s));
    });
    names.into_iter().map(|n| skill(n, rows)).collect()
}
