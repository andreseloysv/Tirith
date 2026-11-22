//! A bot's routine gone worse: its last 5 runs cost 30 % more (in tokens or in dollars) than the 10 before them, or 2 of them failed (of fewer, when it has no five yet; a run that never started is one, with no tokens to compare). The measures' pass emits `routine.worse` for it once per episode (kv `routine_worse:<bot>:<routine>`, cleared when it is no longer worse), with the bot, the routine and those numbers.
use crate::app::Shared;
use crate::server::s;

/// One run of a routine: whether it failed, its tokens (all four kinds) and its cost.
pub(super) struct Run {
    pub(super) failed: bool,
    pub(super) tokens: f64,
    pub(super) cost: Option<f64>,
}

/// The last 15 runs of routine `routine` of the bot whose conversation is `conv`, newest first: a B run of an experiment is no run of the routine, and one that waits for the owner's yes (`waiting`) is no failure.
pub(super) fn runs(app: &Shared, conv: &str, routine: &str) -> Vec<Run> {
    let sql = "SELECT status = 'failed', COALESCE(tokens_in, 0) + COALESCE(tokens_out, 0) + COALESCE(tokens_cache_read, 0) + COALESCE(tokens_cache_write, 0), cost_usd FROM turns WHERE conv = ?1 AND role = ?1 AND routine = ?2 AND COALESCE(variant, '') <> 'B' ORDER BY id DESC LIMIT 15";
    app.query(sql, [conv, routine], |r| {
        Ok(Run {
            failed: r.get(0)?,
            tokens: r.get::<_, i64>(1)? as f64,
            cost: r.get(2)?,
        })
    })
}

/// Of the last 5 runs against the 10 before: the mean of what `f` gives, when it grew by 30 % or more.
fn grew(last: &[Run], before: &[Run], f: fn(&Run) -> Option<f64>) -> Option<(f64, f64)> {
    let mean = |v: &[Run]| {
        let x: Vec<f64> = v.iter().filter_map(f).collect();
        (!x.is_empty()).then(|| x.iter().sum::<f64>() / x.len() as f64)
    };
    (before.len() == 10).then_some(())?;
    let (now, was) = (mean(last)?, mean(before)?);
    (was > 0.0 && now >= was * 1.3).then_some((now, was))
}

/// Why the runs (newest first) went worse, if they did.
pub(super) fn worse(runs: &[Run]) -> Option<String> {
    let (last, before) = runs.split_at(runs.len().min(5)); // two failures need no five runs
    let pct = |(now, was): (f64, f64)| ((now / was - 1.0) * 100.0).round();
    let mut why = vec![];
    if let Some(t) = grew(last, before, |r| (r.tokens > 0.0).then_some(r.tokens)) {
        why.push(format!(
            "tokens per run {:.0} against {:.0} in the 10 before (+{} %)",
            t.0,
            t.1,
            pct(t)
        ));
    }
    if let Some(c) = grew(last, before, |r| r.cost) {
        why.push(format!(
            "$ per run {:.4} against {:.4} in the 10 before (+{} %)",
            c.0,
            c.1,
            pct(c)
        ));
    }
    let failed = last.iter().filter(|r| r.failed).count();
    let _ =
        (failed >= 2).then(|| why.push(format!("{failed} of the last {} runs failed", last.len())));
    (!why.is_empty()).then(|| why.join("; "))
}

/// Every bot's routine with runs, checked: a routine newly worse emits `routine.worse`, one no longer worse ends its episode.
pub(super) fn check(app: &Shared) {
    let pairs = app.query(
        "SELECT DISTINCT conv, routine FROM turns WHERE routine != '' AND conv = role",
        [],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
    );
    for (conv, routine) in pairs {
        let key = format!("routine_worse:{conv}:{routine}");
        let (why, open) = (worse(&runs(app, &conv, &routine)), app.kv(&key).is_some());
        let _ = (why.is_none() && open).then(|| app.set_kv(&key, None));
        let Some(why) = why.filter(|_| !open) else {
            continue;
        };
        app.set_kv(&key, Some(&crate::app::now_iso()));
        let bot = crate::bots::bot(app, &conv).unwrap_or_default();
        let title = bot["routines"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|r| r["id"] == routine.as_str())
            .map(|r| s(r, "title"))
            .unwrap_or_default();
        let text = format!(
            "Routine worse: bot {} ({conv}), routine «{title}» ({routine}): its last runs: {why}.",
            s(&bot, "name")
        );
        crate::manager::emit_event(app, &conv, "routine.worse", &text, None);
    }
}
