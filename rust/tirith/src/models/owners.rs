//! Which agent a model belongs to, and why an agent cannot run a model.
use crate::config::{get, Config};
use std::collections::BTreeMap;
/// The agent a model belongs to, from the configuration alone: its group in `[models.choices]` (a table `alias = [models]`), the agents' catalog, Codex's catalog, or the `model_prefix` its `[agents.<alias>]` declares (by default its command's program name and "-"). None when no agent claims it.
pub fn agent_of(cfg: &Config, model: &str) -> Option<String> {
    grouped_owner(cfg, model)
        .or_else(|| crate::setup::owner_of_model(model))
        .or_else(|| codex_owner(model))
        .or_else(|| prefix_owner(cfg, model))
}
/// The model's group in `[models.choices]`.
fn grouped_owner(cfg: &Config, model: &str) -> Option<String> {
    get::<BTreeMap<String, Vec<String>>>(&cfg.raw, &["models", "choices"])
        .unwrap_or_default()
        .into_iter()
        .find(|(_, ms)| ms.iter().any(|m| m == model))
        .map(|(a, _)| a)
}
/// "codex" when Codex's catalog lists the model.
fn codex_owner(model: &str) -> Option<String> {
    crate::codex::catalog()
        .iter()
        .any(|m| m["slug"] == model)
        .then(|| "codex".to_string())
}
/// The first agent whose (non-empty) model prefix starts the model.
fn prefix_owner(cfg: &Config, model: &str) -> Option<String> {
    cfg.factory
        .commands()
        .into_iter()
        .find(|(a, c)| {
            Some(prefix(cfg, a, c))
                .filter(|p| !p.is_empty())
                .is_some_and(|p| model.starts_with(&p))
        })
        .map(|(a, _)| a)
}
/// The `model_prefix` of an agent, by default its command's program name and "-".
fn prefix(cfg: &Config, alias: &str, command: &str) -> String {
    cfg.factory
        .agent::<String>(alias, "model_prefix")
        .unwrap_or_else(|| format!("{}-", program_name(command)))
}
/// A command's program name without its folder or extension (`codex.exe` on Windows is `codex`).
fn program_name(command: &str) -> &str {
    let file = command.rsplit(['/', '\\']).next().unwrap_or(command);
    file.strip_suffix(".exe")
        .or_else(|| file.strip_suffix(".cmd"))
        .unwrap_or(file)
}
/// Why `agent` cannot run `model`: the model belongs to another agent (an empty model, the agent's default, always fits, and so does any to an agent with `any_model`, which serves several vendors').
pub fn mismatch(cfg: &Config, agent: &str, model: Option<&str>) -> Option<String> {
    let m = model.filter(|m| !m.is_empty())?;
    let any = cfg.factory.agent::<bool>(agent, "any_model") == Some(true);
    (!any).then_some(())?;
    agent_of(cfg, m).filter(|a| a != agent).map(|a| {
        format!(
            "model '{m}' is agent {a}'s, not {agent}'s: choose a model of {agent} or the agent {a}"
        )
    })
}
