//! A role's setting with the owner's changes applied, checked before it is saved.
use super::roles::{role, RoleSetting};
use super::{owners::agent_of, validate};
use crate::app::Shared;
use crate::config::Config;
use serde_json::Value;
/// The role's setting with `b`'s fields applied, or why it cannot be saved.
pub(super) fn updated(app: &Shared, role_name: &str, b: &Value) -> Result<RoleSetting, String> {
    let (mut s, cfg) = (role(app, role_name), app.cfg());
    apply(&cfg, &mut s, b);
    validate::tokens(&s)?;
    validate::aliases(&cfg, &s)?;
    validate::codex_effort(&s)?;
    validate::pairs(&cfg, &s)?;
    Ok(s)
}
/// Applies `b`'s agent, model, effort and fallback; a new agent drops the old model and effort.
fn apply(cfg: &Config, s: &mut RoleSetting, b: &Value) {
    let agent = field(
        b,
        "agent",
        b["model"]
            .as_str()
            .and_then(|m| agent_of(cfg, m.trim()))
            .or(Some(s.agent.clone())),
    )
    .unwrap_or_default(); // a model alone brings its own agent
    let _ = (agent != s.agent).then(|| (s.model, s.effort) = (None, None));
    s.agent = agent;
    s.model = field(b, "model", s.model.take());
    s.effort = field(b, "effort", s.effort.take());
    if let Some(f) = fallback(&b["fallback"]) {
        s.fallback = f;
    }
}
/// A string field of `b`: blank clears it, absent keeps `cur`.
fn field(b: &Value, k: &str, cur: Option<String>) -> Option<String> {
    match &b[k] {
        Value::String(v) if v.trim().is_empty() => None,
        Value::String(v) => Some(v.trim().to_string()),
        _ => cur,
    }
}
/// The fallback chain from a list or a comma-separated string; None keeps the current one.
fn fallback(v: &Value) -> Option<Vec<String>> {
    match v {
        Value::Array(a) => Some(
            a.iter()
                .filter_map(|x| x.as_str())
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect(),
        ),
        Value::String(x) => Some(
            x.split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect(),
        ),
        _ => None,
    }
}
