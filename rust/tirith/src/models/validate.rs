//! The checks a role's setting passes before it is saved.
use super::{owners::mismatch, roles::RoleSetting};
use crate::config::Config;
/// Every model, effort and fallback entry is a token.
pub(super) fn tokens(s: &RoleSetting) -> Result<(), String> {
    [&s.model, &s.effort]
        .into_iter()
        .flatten()
        .chain(&s.fallback)
        .find(|v| !token(v))
        .map_or(Ok(()), |v| {
            Err(format!(
                "'{v}' is not a valid model, effort or fallback entry"
            ))
        })
}
/// The agent and every fallback's alias are agents of config.toml.
pub(super) fn aliases(cfg: &Config, s: &RoleSetting) -> Result<(), String> {
    std::iter::once(&s.agent)
        .chain(&s.fallback)
        .map(|e| e.split(':').next().unwrap_or(""))
        .find(|a| cfg.factory.command(a).is_none())
        .map_or(Ok(()), |alias| {
            Err(format!("no agent alias '{alias}' in config.toml"))
        })
}
/// A Codex model lists its efforts: the chosen effort is one of them.
pub(super) fn codex_effort(s: &RoleSetting) -> Result<(), String> {
    let listed = (s.agent == "codex")
        .then(crate::codex::catalog)
        .and_then(|c| {
            c.into_iter()
                .find(|m| m["slug"].as_str() == s.model.as_deref())
        });
    listed
        .filter(|m| {
            s.effort.as_ref().is_some_and(|e| {
                !m["efforts"]
                    .as_array()
                    .is_some_and(|es| es.iter().any(|v| v == e))
            })
        })
        .map_or(Ok(()), |m| {
            Err(format!(
                "unsupported effort for {}: {}",
                m["slug"], m["efforts"]
            ))
        })
}
/// Neither the agent nor a fallback is paired with another agent's model.
pub(super) fn pairs(cfg: &Config, s: &RoleSetting) -> Result<(), String> {
    std::iter::once(format!("{}:{}", s.agent, s.model.as_deref().unwrap_or("")))
        .chain(s.fallback.clone())
        .find_map(|e| {
            e.split_once(':')
                .and_then(|(a, m)| mismatch(cfg, a, Some(m)))
        })
        .map_or(Ok(()), Err)
}
/// A model, effort, fallback entry or request id: a letter or digit first, then only those and `._:/+-[]`.
pub(super) fn token(v: &str) -> bool {
    v.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._:/+-[]".contains(c))
}
