//! The captured runs a replay does again: the routine's last ones whose outside reads can all be answered, and enough of them to prove something.
use crate::app::App;

/// For a replay, the turns of those runs, oldest first, after the bot's folder is committed so A is its main as it is now. None for the other modes.
pub(super) fn runs(app: &App, key: &str, routine: &str, mode: &str) -> Result<Vec<i64>, String> {
    if mode != "replay" {
        return Ok(vec![]);
    }
    crate::bots::versioned(app, key, "before the replay");
    let lab = app.cfg().factory.lab.clone();
    let all = crate::turns::captured(app, key, routine, lab.inputs as usize);
    if all.is_empty() {
        return Err(format!(
            "routine '{routine}' has no captured run to replay yet"
        ));
    }
    let answerable = |turn: &i64| {
        crate::turns::capture_of(app, *turn).is_some_and(|c| super::recorded::answerable(&c.calls))
    };
    let runs: Vec<i64> = all.into_iter().filter(answerable).collect();
    let least = super::due::minimum(&lab);
    if (runs.len() as i64) < least {
        return Err(format!(
            "routine '{routine}' has {} captured run(s) that can be replayed and a replay needs {least}: use live or shadow",
            runs.len()
        ));
    }
    Ok(runs)
}
