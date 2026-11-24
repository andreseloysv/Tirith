//! The factory's `[lab]` rule applied to an experiment's numbers, and the one line that says what NeoBot decided.
use super::saved::{counts, savings, shown};
use crate::factory::Lab;
use serde::Serialize;

/// One side's runs: how many, and their mean tokens, dollars (when every run knows them) and milliseconds.
#[derive(Serialize, Default, Clone)]
pub struct Side {
    pub runs: i64,
    pub tokens: f64,
    pub cost: Option<f64>,
    pub ms: f64,
}

/// B's comparisons with A (won, tied, lost), the runs where B failed and A did not, B's 👎 and the replay pairs that proved nothing (`voids`: counted in none of the others).
#[derive(Serialize, Default, Clone)]
pub struct Tally {
    /// What each pair saved by itself: in time, in tokens, in dollars.
    pub pair_times: Vec<f64>,
    pub pair_tokens: Vec<f64>,
    pub pair_costs: Vec<f64>,
    pub wins: i64,
    pub ties: i64,
    pub losses: i64,
    pub new_failures: i64,
    pub down: i64,
    pub voids: i64,
}

/// The rule: B is adopted when it is no worse (wins and ties at least `not_worse_share` of the comparisons, and wins at least as many as it loses), has no new failure and no 👎, and saves at least `min_saving` of A's mean cost (tokens or $) or time (`saved.rs`: in most pairs too). Whether it is, and the line that says so for `name` (the routine and the branch).
pub fn decide(lab: &Lab, t: &Tally, a: &Side, b: &Side, name: &str) -> (bool, String) {
    let n = t.wins + t.ties + t.losses;
    let share = (t.wins + t.ties) as f64 / n.max(1) as f64;
    let no_worse = n > 0 && share >= lab.not_worse_share && t.wins >= t.losses;
    let all = savings(a, b);
    let counted = all.iter().copied().find(|s| counts(lab, t, *s));
    let (saves, best) = (counted.is_some(), counted.or(all.first().copied()));
    let uneven = !saves && best.is_some_and(|b| b.1 >= lab.min_saving);
    let quality = match (no_worse, t.wins > t.losses) {
        (true, true) => "better quality",
        (true, false) => "same quality",
        _ => "worse quality",
    };
    let quality = format!("{quality} ({}/{n})", t.wins + t.ties);
    let saving = best.map_or("no cost numbers".into(), shown);
    let least = super::due::minimum(lab);
    let why = if n == 0 {
        "no comparisons".to_string()
    } else if n < least {
        format!("only {n} comparisons (minimum {least})")
    } else if t.new_failures > 0 {
        format!("{} new failure(s)", t.new_failures)
    } else if t.down > 0 {
        format!("{} 👎", t.down)
    } else if no_worse && uneven {
        format!("saves only in some pair ({saving})")
    } else if no_worse && !saves {
        format!("saves little ({saving})")
    } else {
        String::new()
    };
    let adopt = no_worse && saves && why.is_empty();
    let line = if adopt {
        format!("Adopted {name}: {saving}, {quality}")
    } else if n == 0 {
        format!("Discarded {name}: {why}")
    } else if why.is_empty() {
        format!("Discarded {name}: {quality}, {saving}")
    } else {
        format!("Discarded {name}: {why}, {quality}")
    };
    let voids = (t.voids > 0).then(|| {
        format!(
            " · {} pair(s) not compared: they failed, used another model or asked for something the capture lacks",
            t.voids
        )
    });
    (adopt, line + &voids.unwrap_or_default())
}
