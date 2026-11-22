//! The turn's own numbers from an agent that gives its session's so far: Anthropic's cost, Codex's and Copilot's tokens. What a conversation's session had spent before is kept in kv `spent:<conversation>`.
use super::record::Turn;
use crate::app::App;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone)]
struct SoFar {
    /// Empty: the conversation's next session is a new one, so all it says is the turn's.
    session: String,
    input: i64,
    output: i64,
    cache_read: i64,
    cache_write: i64,
    cost: f64,
}

fn key(conv: &str) -> String {
    format!("spent:{conv}")
}

fn keep(app: &App, conv: &str, now: &SoFar) {
    app.set_kv(&key(conv), serde_json::to_string(now).ok().as_deref());
}

/// The conversation's agent starts a new session: it has spent nothing.
pub fn fresh(app: &App, conv: &str) {
    keep(app, conv, &SoFar::default());
}

/// The turn's share of its session's totals. A turn of its own (`loose`) is its whole session; a session whose spending before is not known gives none rather than a guess.
pub(super) fn own_share(app: &App, t: &mut Turn) {
    let (totals, u) = (t.usage.session_totals, &mut t.usage);
    if t.loose || !u.seen || !(totals || u.cost.is_some()) {
        return;
    }
    let now = SoFar {
        session: t.session.clone(),
        input: if totals { u.input } else { 0 },
        output: if totals { u.output } else { 0 },
        cache_read: if totals { u.cache_read } else { 0 },
        cache_write: if totals { u.cache_write } else { 0 },
        cost: u.cost.unwrap_or(0.0),
    };
    let before = app
        .kv(&key(&t.conv))
        .and_then(|v| serde_json::from_str::<SoFar>(&v).ok())
        .filter(|b| b.session.is_empty() || b.session == now.session);
    keep(app, &t.conv, &now);
    let Some(before) = before else {
        (u.cost, u.seen) = (None, !totals);
        if totals {
            (u.input, u.output, u.cache_read, u.cache_write) = (0, 0, 0, 0);
        }
        return;
    };
    // Smaller than before: the agent began to count again.
    if totals && now.input >= before.input && now.output >= before.output {
        u.input -= before.input;
        u.output -= before.output;
        u.cache_read = (u.cache_read - before.cache_read).max(0);
        u.cache_write = (u.cache_write - before.cache_write).max(0);
    }
    if let Some(cost) = u.cost.filter(|c| *c >= before.cost) {
        u.cost = Some(cost - before.cost);
    }
}
