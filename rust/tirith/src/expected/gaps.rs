//! When Tirith was away: it keeps that it is alive, so what was missed says whether Tirith was stopped or the machine slept.
use super::calendar::local;
use super::words::when;
use crate::app::App;
use crate::server::s;
use chrono::{DateTime, Local};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Mutex;

const ALIVE: &str = "expected:alive";
const GAPS: &str = "expected:gaps";
/// No beat for this long is being away (one comes every 30 s or so).
const AWAY: i64 = 150;
const KEPT: usize = 20;
/// The homes this process beat for: its first beat follows a start.
static BEATING: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());

/// Tirith is alive now; a hole since the last time is kept, with whether this process had just started.
pub(super) fn beat(app: &App, now: DateTime<Local>) {
    let just_started = BEATING.lock().unwrap().insert(app.home.clone());
    let now_kept = super::calendar::utc(now);
    let before = app.kv(ALIVE);
    let hole = before
        .as_deref()
        .and_then(local)
        .is_some_and(|b| (now - b).num_seconds() > AWAY);
    if hole {
        let mut gaps = gaps(app);
        gaps.push(json!({"from": before, "to": now_kept, "stopped": just_started}));
        let from = gaps.len().saturating_sub(KEPT);
        app.set_kv(GAPS, Some(&json!(gaps[from..]).to_string()));
    }
    app.set_kv(ALIVE, Some(&now_kept));
}

fn gaps(app: &App) -> Vec<Value> {
    app.kv(GAPS)
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default()
}

/// Why nothing could start at `due`, when Tirith was away then.
pub(super) fn over(app: &App, due: &str) -> Option<String> {
    let gap = gaps(app)
        .into_iter()
        .find(|g| s(g, "from").as_str() <= due && due <= s(g, "to").as_str())?;
    let span = format!("de {} a {}", when(&s(&gap, "from")), when(&s(&gap, "to")));
    Some(if gap["stopped"] == true {
        format!("Tirith estaba parado ({span})")
    } else {
        format!("la máquina dormía o Tirith no respondía ({span})")
    })
}

/// What the line of a run that starts late adds.
pub fn late(app: &App, due: &str, now: DateTime<Local>) -> String {
    let Some(was) = local(due).filter(|t| (now - *t).num_seconds() > AWAY) else {
        return String::new();
    };
    let why = over(app, due).map(|w| format!(", {w}")).unwrap_or_default();
    let was = if was.date_naive() == now.date_naive() {
        format!("a las {}", was.format("%H:%M"))
    } else {
        format!("el {}", super::words::at(was))
    };
    format!(" · tarde: tocaba {was}{why}")
}
