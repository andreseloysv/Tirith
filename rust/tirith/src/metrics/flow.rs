//! How the merged lanes flowed: the median minutes from start to main, and the share merged at the first try.
use super::compute::Ctx;
use super::metric::{ids, secs, Metric};
use super::work::merged_end;
use crate::app::Lane;
pub(super) fn median(c: &Ctx, merged: &[&Lane]) -> Metric {
    let mut mins: Vec<(f64, &str)> = merged
        .iter()
        .filter_map(|l| {
            Some((
                (merged_end(l)? - secs(&l.created_at)?) as f64 / 60.0,
                l.id.as_str(),
            ))
        })
        .collect();
    mins.sort_by(|a, b| a.0.total_cmp(&b.0));
    let median = if mins.is_empty() {
        0.0
    } else {
        (mins[(mins.len() - 1) / 2].0 + mins[mins.len() / 2].0) / 2.0
    };
    c.mk(
        "median",
        median,
        !mins.is_empty(),
        mins.iter()
            .rev()
            .take(8)
            .map(|(m, id)| format!("{id} {m:.0} min"))
            .collect::<Vec<_>>()
            .join(", "),
    )
}
pub(super) fn first_try(c: &Ctx, merged: &[&Lane]) -> Metric {
    let pct = if merged.is_empty() {
        100.0
    } else {
        (100 * merged.iter().filter(|l| l.attempts == 0).count() / merged.len()) as f64
    };
    let retried: Vec<&Lane> = merged.iter().filter(|l| l.attempts > 0).copied().collect();
    c.mk(
        "first_try",
        pct,
        !merged.is_empty(),
        format!("merged after being sent back: [{}]", ids(&retried)),
    )
}
