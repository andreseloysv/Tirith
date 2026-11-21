//! The checks that a factory's names point at something: protocols, parents, agents, prompts and measures.
use super::lists::MEASURES;
use super::values::{stem, str_of};
use std::collections::BTreeMap;

type Section = toml::Table;
type Prompts = BTreeMap<String, String>;

const SHAPES: [(&str, &[&str]); 4] = [
    ("session", &["chat", "turn", "lane"]),
    ("scope", &["project", "own"]),
    ("access", &["", "read", "write"]),
    ("runs_on", &["", "local"]),
];

/// The first agent whose protocol is not one the code speaks.
pub(super) fn bad_protocol(agents: &Section) -> Option<String> {
    agents
        .iter()
        .find(|(_, a)| !matches!(str_of(a, "protocol"), "" | "stream-json" | "exec"))
        .map(|(n, a)| {
            format!(
                "agent '{n}' has unknown protocol '{}'",
                str_of(a, "protocol")
            )
        })
}

/// The first role whose `extends` stayed after resolving: its parent is missing or a cycle.
pub(super) fn missing_parent(roles: &Section) -> Option<String> {
    roles
        .iter()
        .find(|(_, r)| r.get("extends").is_some())
        .map(|(n, _)| format!("role '{n}' extends a missing role"))
}

/// The first role that runs on an agent the factory does not have.
pub(super) fn missing_agent(roles: &Section, agents: &Section) -> Option<String> {
    roles.iter().find_map(|(n, r)| {
        r.get("agent")
            .and_then(|x| x.as_str())
            .filter(|a| !agents.contains_key(*a))
            .map(|a| format!("role '{n}' runs on missing agent '{a}'"))
    })
}

/// The first role with a session, scope, access or runs_on outside its closed list.
pub(super) fn bad_shape(roles: &Section) -> Option<String> {
    roles
        .iter()
        .find(|(_, r)| SHAPES.iter().any(|(k, ok)| !ok.contains(&str_of(r, k))))
        .map(|(n, r)| {
            format!(
                "role '{n}' has unknown session '{}', scope '{}', access '{}' or runs_on '{}'",
                str_of(r, "session"),
                str_of(r, "scope"),
                str_of(r, "access"),
                str_of(r, "runs_on")
            )
        })
}

/// The first prompt file a role names that the factory does not hold.
pub(super) fn missing_prompt(roles: &Section, prompts: &Prompts) -> Option<String> {
    roles.iter().find_map(|(n, r)| {
        r.get("prompt")
            .and_then(|p| p.as_array())
            .into_iter()
            .flatten()
            .filter_map(|p| p.as_str())
            .find(|p| !prompts.contains_key(&stem(p)))
            .map(|p| format!("role '{n}' names missing prompt '{p}'"))
    })
}

/// The first `[metrics.*]` that is not one of the measures the code computes.
pub(super) fn unknown_measure(metrics: Option<&Section>) -> Option<String> {
    metrics.and_then(|m| {
        m.keys()
            .find(|id| !MEASURES.contains(&id.as_str()))
            .map(|id| format!("unknown measure '{id}'"))
    })
}
