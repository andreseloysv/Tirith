//! What B saves against A, and which saving counts.
use super::rule::{Side, Tally};
use crate::factory::Lab;

const TIME: &str = "tiempo";

/// What B saves against A, as a share of A's mean, by what it is measured in (tokens, $ or time), the largest first.
pub fn savings(a: &Side, b: &Side) -> Vec<(&'static str, f64)> {
    let less = |a: f64, b: f64| (a > 0.0).then(|| 1.0 - b / a);
    let cost = a.cost.zip(b.cost).and_then(|(a, b)| less(a, b));
    let mut out: Vec<_> = [
        ("tokens", less(a.tokens, b.tokens)),
        ("$", cost),
        (TIME, less(a.ms, b.ms)),
    ]
    .into_iter()
    .filter_map(|(k, v)| v.map(|v| (k, v)))
    .collect();
    out.sort_by(|x, y| y.1.total_cmp(&x.1));
    out
}

/// Whether a saving is one: at least `min_saving`, and time, which one slow answer of a model moves, also in most of the pairs one by one.
pub(super) fn counts(lab: &Lab, t: &Tally, (what, share): (&str, f64)) -> bool {
    let quicker = t
        .pair_times
        .iter()
        .filter(|s| **s >= lab.min_saving)
        .count();
    let steady = t.pair_times.is_empty() || quicker * 2 > t.pair_times.len();
    share >= lab.min_saving && (what != TIME || steady)
}

/// A saving as the owner reads it: «−38 % tokens», «+12 % tiempo».
pub(super) fn shown((what, share): (&str, f64)) -> String {
    let pct = (share * 100.0).round() as i64;
    let sign = if pct > 0 { "−" } else { "+" };
    format!("{sign}{} % {what}", pct.abs())
}
