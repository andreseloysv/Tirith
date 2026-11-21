//! The strict check of one factory version: the first thing wrong in it, in a fixed order, makes it invalid.
use super::keys::{AGENT_KEYS, EVAL_KEYS, ROLE_KEYS, ROUTE_KEYS};
use super::loaded::Factory;
use super::refs::{
    bad_protocol, bad_shape, missing_agent, missing_parent, missing_prompt, unknown_measure,
};
use super::rowrefs::{bad_eval, bad_route};
use super::unknown::{sub, unknown_flat, unknown_in, unknown_keys, unknown_section};
use super::values::rows;

impl Factory {
    pub(super) fn validate(&self) -> Result<(), String> {
        self.unknown_key()
            .or_else(|| self.missing_name())
            .map_or(Ok(()), |e| Err(format!("factory: {e}")))
    }

    /// The first section or key nothing reads.
    fn unknown_key(&self) -> Option<String> {
        let t = &self.raw;
        unknown_section(t)
            .or_else(|| unknown_in(t, "agents", AGENT_KEYS))
            .or_else(|| unknown_in(t, "roles", ROLE_KEYS))
            .or_else(|| unknown_in(t, "metrics", &["label", "target", "at_least"]))
            .or_else(|| unknown_flat(t, "plan", &["role_prefixes", "owner_wait", "waits_for"]))
            .or_else(|| unknown_flat(t, "cards", &["kinds"]))
    }

    /// The first name that points at nothing, with the unknown keys of routes and evals in their place.
    fn missing_name(&self) -> Option<String> {
        let t = &self.raw;
        let agents = sub(t, "agents").cloned().unwrap_or_default();
        let roles = sub(t, "roles").cloned().unwrap_or_default();
        let routes: Vec<toml::Value> = rows(t, "routes").cloned().collect();
        let evals: Vec<toml::Value> = rows(t, "evals").cloned().collect();
        bad_protocol(&agents)
            .or_else(|| missing_parent(&roles))
            .or_else(|| missing_agent(&roles, &agents))
            .or_else(|| bad_shape(&roles))
            .or_else(|| missing_prompt(&roles, &self.prompts))
            .or_else(|| unknown_measure(sub(t, "metrics")))
            .or_else(|| unknown_keys(&routes, ROUTE_KEYS, "routes"))
            .or_else(|| bad_route(&routes, &roles))
            .or_else(|| unknown_keys(&evals, EVAL_KEYS, "evals"))
            .or_else(|| bad_eval(&evals, &roles, &self.prompts))
    }
}
