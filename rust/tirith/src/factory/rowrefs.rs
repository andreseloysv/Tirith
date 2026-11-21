//! The checks that a factory's routes and evals name a real event, role, mode, rubric and sample.
use super::lists::is_event;
use super::values::{stem, str_of};
use std::collections::BTreeMap;

/// The first route on a missing event, to or then a missing role, or with an unknown mode.
pub(super) fn bad_route(routes: &[toml::Value], roles: &toml::Table) -> Option<String> {
    let is_role = |n: &str| n.is_empty() || roles.contains_key(n);
    routes
        .iter()
        .find(|r| {
            !is_event(str_of(r, "on"))
                || !is_role(str_of(r, "to"))
                || !is_role(str_of(r, "then"))
                || !matches!(str_of(r, "mode"), "" | "turn" | "note")
        })
        .map(|r| {
            format!(
                "route on '{}' names a missing event, role or mode ('{}', '{}')",
                str_of(r, "on"),
                str_of(r, "to"),
                str_of(r, "mode")
            )
        })
}

/// The first eval with no id, a missing event, judge role or rubric, or a sample outside 0 to 1.
pub(super) fn bad_eval(
    evals: &[toml::Value],
    roles: &toml::Table,
    prompts: &BTreeMap<String, String>,
) -> Option<String> {
    evals
        .iter()
        .find(|e| {
            str_of(e, "id").is_empty()
                || !is_event(str_of(e, "on"))
                || !roles.contains_key(str_of(e, "judge"))
                || !prompts.contains_key(&stem(str_of(e, "rubric")))
                || !(0.0..=1.0).contains(&sample(e))
        })
        .map(|e| {
            format!(
                "eval '{}' names a missing event, judge role or rubric, or a sample outside 0 to 1 ('{}', '{}', '{}')",
                str_of(e, "id"),
                str_of(e, "on"),
                str_of(e, "judge"),
                str_of(e, "rubric")
            )
        })
}

/// An eval's sample share, all of them when it names none.
pub(super) fn sample(e: &toml::Value) -> f64 {
    e.get("sample").and_then(|v| v.as_float()).unwrap_or(1.0)
}
