//! A measure's red episode, kept across restarts.
use crate::app::Shared;
/// One metric's red episode — since when (the manager was told then) and whether its review ran — kept in `kv` (`metric_red:<id>` = "<since>@<reviewed>") so a daemon restart neither announces `metric.red` again nor repeats the review of a measure already red; back to green clears it. Returns (fresh, review due, since).
pub(super) fn episode(
    app: &Shared,
    id: &str,
    red: bool,
    now: u64,
    after: u64,
) -> (bool, bool, Option<u64>) {
    let key = format!("metric_red:{id}");
    let cur = app.kv(&key).and_then(|v| {
        v.split_once('@')
            .and_then(|(s, r)| s.parse::<u64>().ok().map(|s| (s, r == "1")))
    });
    let (since, reviewed) = cur.unwrap_or((now, false));
    let due = red && cur.is_some() && !reviewed && now >= since.saturating_add(after);
    let _ = (!red).then(|| app.set_kv(&key, None));
    let _ = (red && (cur.is_none() || due))
        .then(|| app.set_kv(&key, Some(&format!("{since}@{}", due as u8))));
    (red && cur.is_none(), due, red.then_some(since))
}
