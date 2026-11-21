//! The factory's `[lab]` rule applied to an experiment's numbers, and the one line that says what Tirith decided.
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

/// B's comparisons with A (won, tied, lost), the runs where B failed and A did not, and B's 👎.
#[derive(Serialize, Default, Clone)]
pub struct Tally {
    pub wins: i64,
    pub ties: i64,
    pub losses: i64,
    pub new_failures: i64,
    pub down: i64,
}

/// What B saves against A, as a share of A's mean, by what it is measured in (tokens, $ or time), the largest first.
pub fn savings(a: &Side, b: &Side) -> Vec<(&'static str, f64)> {
    let less = |a: f64, b: f64| (a > 0.0).then(|| 1.0 - b / a);
    let cost = a.cost.zip(b.cost).and_then(|(a, b)| less(a, b));
    let mut out: Vec<_> = [
        ("tokens", less(a.tokens, b.tokens)),
        ("$", cost),
        ("tiempo", less(a.ms, b.ms)),
    ]
    .into_iter()
    .filter_map(|(k, v)| v.map(|v| (k, v)))
    .collect();
    out.sort_by(|x, y| y.1.total_cmp(&x.1));
    out
}

/// A saving as the owner reads it: «−38 % tokens», «+12 % tiempo».
fn shown((what, share): (&str, f64)) -> String {
    let pct = (share * 100.0).round() as i64;
    let sign = if pct > 0 { "−" } else { "+" };
    format!("{sign}{} % {what}", pct.abs())
}

/// The rule: B is adopted when it is no worse (wins and ties at least `not_worse_share` of the comparisons, and wins at least as many as it loses), has no new failure and no 👎, and saves at least `min_saving` of A's mean cost (tokens or $) or time. Whether it is, and the line that says so for `name` (the routine and the branch).
pub fn decide(lab: &Lab, t: &Tally, a: &Side, b: &Side, name: &str) -> (bool, String) {
    let n = t.wins + t.ties + t.losses;
    let share = (t.wins + t.ties) as f64 / n.max(1) as f64;
    let no_worse = n > 0 && share >= lab.not_worse_share && t.wins >= t.losses;
    let best = savings(a, b).into_iter().next();
    let saves = best.is_some_and(|b| b.1 >= lab.min_saving);
    let quality = match (no_worse, t.wins > t.losses) {
        (true, true) => "mejor calidad",
        (true, false) => "misma calidad",
        _ => "peor calidad",
    };
    let quality = format!("{quality} ({}/{n})", t.wins + t.ties);
    let saving = best.map_or("sin números de coste".into(), shown);
    let why = if n == 0 {
        "sin comparaciones".to_string()
    } else if t.new_failures > 0 {
        format!("{} fallo(s) nuevo(s)", t.new_failures)
    } else if t.down > 0 {
        format!("{} 👎", t.down)
    } else if no_worse && !saves {
        format!("ahorra poco ({saving})")
    } else {
        String::new()
    };
    let adopt = no_worse && saves && why.is_empty();
    let line = if adopt {
        format!("Adopté {name}: {saving}, {quality}")
    } else if n == 0 {
        format!("Descarté {name}: {why}")
    } else if why.is_empty() {
        format!("Descarté {name}: {quality}, {saving}")
    } else {
        format!("Descarté {name}: {why}, {quality}")
    };
    (adopt, line)
}
