//! What B saves against A, and which saving counts.
use super::rule::{Side, Tally};
use crate::factory::Lab;

const TIME: &str = "tiempo";
const TOKENS: &str = "tokens";

/// What `b` is less than `a`, as a share of `a`.
pub(super) fn less(a: f64, b: f64) -> Option<f64> {
    (a > 0.0).then(|| 1.0 - b / a)
}

/// What B saves against A, as a share of A's mean, by what it is measured in (tokens, $ or time), the largest first.
pub fn savings(a: &Side, b: &Side) -> Vec<(&'static str, f64)> {
    let cost = a.cost.zip(b.cost).and_then(|(a, b)| less(a, b));
    let mut out: Vec<_> = [
        (TOKENS, less(a.tokens, b.tokens)),
        ("$", cost),
        (TIME, less(a.ms, b.ms)),
    ]
    .into_iter()
    .filter_map(|(k, v)| v.map(|v| (k, v)))
    .collect();
    out.sort_by(|x, y| y.1.total_cmp(&x.1));
    out
}

/// Whether a saving is one: at least `min_saving` of the mean, which one slow answer or one tool call too many moves, and also in most of the pairs one by one.
pub(super) fn counts(lab: &Lab, t: &Tally, (what, share): (&str, f64)) -> bool {
    let each = match what {
        TIME => &t.pair_times,
        TOKENS => &t.pair_tokens,
        _ => &t.pair_costs,
    };
    let saved = each.iter().filter(|s| **s >= lab.min_saving).count();
    share >= lab.min_saving && (each.is_empty() || saved * 2 > each.len())
}

/// A saving as the owner reads it: «−38 % tokens», «+12 % tiempo».
pub(super) fn shown((what, share): (&str, f64)) -> String {
    let pct = (share * 100.0).round() as i64;
    let sign = if pct > 0 { "−" } else { "+" };
    format!("{sign}{} % {what}", pct.abs())
}
