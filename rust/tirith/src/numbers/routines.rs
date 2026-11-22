//! A bot's numbers by routine: its runs, success, time, cost per good run, 👎 and corrections per 10, mean score and trend.
use super::rows::Row;
use serde_json::{json, Value};

/// The mean of some numbers, or none without any.
pub fn mean(v: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, n) = v.fold((0.0, 0), |(s, n), x| (s + x, n + 1));
    (n > 0).then(|| sum / n as f64)
}

/// What all the runs cost over the ones that ended ok: none without a good run or without any known cost.
pub fn cost_per_good(rows: &[&Row]) -> Option<f64> {
    let good = rows.iter().filter(|r| r.ok).count();
    let known = rows.iter().any(|r| r.cost.is_some());
    let all: f64 = rows.iter().filter_map(|r| r.cost).sum();
    (good > 0 && known).then(|| all / good as f64)
}

/// Cost per good run, mean time and success of some runs.
fn window(rows: &[&Row]) -> Value {
    let success = mean(
        rows.iter()
            .filter(|r| !r.waiting)
            .map(|r| f64::from(u8::from(r.ok))),
    ); // a run that waits for the owner's yes is neither
    let ms = mean(rows.iter().map(|r| r.ms as f64));
    json!({"runs": rows.len(), "cost_per_good": cost_per_good(rows), "ms": ms, "success": success})
}

/// The last 10 runs against the 10 before them; none until there are runs before the last 10.
fn trend(rows: &[&Row]) -> Value {
    let n = rows.len();
    let Some(()) = (n > 10).then_some(()) else {
        return Value::Null;
    };
    let (last, before) = (&rows[n - 10..], &rows[n.saturating_sub(20)..n - 10]);
    json!({"last": window(last), "before": window(before)})
}

/// How many of `rows` hold `f`, per 10 runs.
fn per10(rows: &[&Row], f: impl Fn(&Row) -> bool) -> f64 {
    let hits = rows.iter().filter(|r| f(r)).count() as f64;
    (hits * 100.0 / rows.len().max(1) as f64).round() / 10.0
}

/// One routine's numbers (`id` empty: the bot's chat turns, no routine).
fn routine(id: &str, title: &str, rows: &[&Row]) -> Value {
    let all = window(rows);
    json!({
        "id": id,
        "title": title,
        "runs": rows.len(),
        "waiting": rows.iter().filter(|r| r.waiting).count(),
        "success": all["success"],
        "ms": all["ms"],
        "cost_per_good": all["cost_per_good"],
        "down_per_10": per10(rows, |r| r.reaction == Some(-1)),
        "corrections_per_10": per10(rows, |r| r.corrected),
        "score": mean(rows.iter().filter_map(|r| r.score)),
        "scored": rows.iter().filter(|r| r.score.is_some()).count(),
        "trend": trend(rows),
    })
}

/// Every routine of the bot (`routines`: its id and title, in the card's order) with runs or not, then routines it no longer has, then its chat turns.
pub fn by_routine(rows: &[Row], routines: &[(String, String)]) -> Vec<Value> {
    let of = |id: &str| rows.iter().filter(|r| r.routine == id).collect::<Vec<_>>();
    let mut ids: Vec<(String, String)> = routines.to_vec();
    for r in rows.iter().filter(|r| !r.routine.is_empty()) {
        let known = ids.iter().any(|(id, _)| *id == r.routine);
        let _ = (!known).then(|| ids.push((r.routine.clone(), format!("{} (removed)", r.routine))));
    }
    let mut out: Vec<Value> = ids.iter().map(|(id, t)| routine(id, t, &of(id))).collect();
    let chat = of("");
    out.extend((!chat.is_empty()).then(|| routine("", "Chat", &chat)));
    out
}
