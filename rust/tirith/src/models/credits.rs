//! Running out of credits, and the next choice of a role's fallback chain.
use super::roles::role;
use crate::app::Shared;
use regex::Regex;
use std::sync::OnceLock;
/// True when an agent's output says it ran out of credits or hit its usage limit.
pub fn out_of_credits(text: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(concat!(
            r"(?i)credit balance is too low|insufficient[ _-]?(credits?|quota|funds|balance)|out of credits",
            r"|no credits? (left|remaining)|(usage|spending|session|weekly|daily|monthly|[0-9]+-hour) limit (reached|exceeded|hit)",
            r"|(hit|reached|exceeded) (your|the) (usage |spending |current |monthly )?(limit|quota)|quota (exceeded|exhausted)",
            r"|quota_(exceeded|exhausted)|overage_limit_reached|no remaining quota|used (all|up) (of )?your [^\n]{0,60}(requests|credits|quota)",
            r"|(hit|reached|exceeded) your (session|weekly|daily|monthly|additional usage) (rate )?limit",
            r"|resource[_ ]exhausted|payment required|\b402\b[^\n]{0,40}(payment|credit|billing)|run out of (credits|usage|quota)",
        ))
        .unwrap()
    })
    .is_match(text)
}
/// The next choice of a role's chain (its own agent, then `fallback`) that is not in `tried`: (alias, model). `tried` holds "alias" (every model of it is skipped) or "alias:model" (that one only).
pub fn fallback(
    app: &Shared,
    role_name: &str,
    tried: &[String],
) -> Option<(String, Option<String>)> {
    let cfg = app.cfg();
    let r = role(app, role_name);
    let first = r
        .model
        .as_ref()
        .map(|m| format!("{}:{m}", r.agent))
        .unwrap_or(r.agent.clone());
    std::iter::once(first).chain(r.fallback).find_map(|e| {
        let (alias, model) = entry(&e);
        let used = tried.iter().any(|t| *t == e || *t == alias);
        (!used && cfg.factory.command(&alias).is_some()).then_some((alias, model))
    })
}
/// A chain entry "alias" or "alias:model" as (alias, model).
fn entry(e: &str) -> (String, Option<String>) {
    e.split_once(':')
        .map(|(a, m)| (a.to_string(), Some(m.to_string())))
        .unwrap_or((e.to_string(), None))
}
