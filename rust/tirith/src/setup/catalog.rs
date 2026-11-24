//! The agents the guide knows (assets/agents.toml, and what a release build lays over it).
use serde::Deserialize;

const TEXT: &str = include_str!("../../assets/agents.toml");
const MORE: &str = include_str!(concat!(env!("OUT_DIR"), "/agents-more.toml"));

#[derive(Deserialize, Clone, Default, Debug)]
#[serde(default)]
pub struct Known {
    #[serde(skip)]
    pub alias: String,
    pub name: String,
    pub order: u32,
    /// Its program's name; empty when only config.toml or the owner says it.
    pub bin: String,
    /// Its maker's installer, a script.
    pub install: String,
    /// Its maker's installer on Windows, a PowerShell script.
    pub install_ps1: String,
    pub installer_env: toml::Table,
    /// Its maker's npm package, installed instead on musl (`npm.rs`).
    pub npm_on_musl: String,
    pub signin: Vec<String>,
    pub status: Vec<String>,
    pub settings: String,
    pub new_settings: toml::Table,
    pub hooks: String,
    /// The variable that signs it in without a login, and where its value comes from.
    pub token_env: String,
    pub token_hint: String,
    /// The variable and the login's flag that point it at an enterprise's GitHub host (`host.rs`).
    pub host_env: String,
    pub host_flag: String,
    /// Hosts its model needs beyond the factory's `model_hosts`.
    pub model_hosts: Vec<String>,
    pub check_model: String,
    pub check_effort: String,
}

/// Every agent, in the guide's order.
pub fn all() -> Vec<Known> {
    read(TEXT, MORE).unwrap_or_else(|_| read(TEXT, "").unwrap_or_default())
}

pub(super) fn read(text: &str, more: &str) -> Result<Vec<Known>, String> {
    let parse = |t: &str| t.parse::<toml::Table>().map_err(|e| e.to_string());
    let mut table = toml::Value::Table(parse(text)?);
    crate::factory::merge(&mut table, &toml::Value::Table(parse(more)?));
    let mut all = vec![];
    for (alias, entry) in table.as_table().into_iter().flatten() {
        let mut known: Known = entry
            .clone()
            .try_into()
            .map_err(|e| format!("agents.toml [{alias}]: {e}"))?;
        known.alias = alias.clone();
        all.push(known);
    }
    all.sort_by_key(|k| k.order);
    Ok(all)
}

pub fn model_hosts(alias: &str) -> Vec<String> {
    known(alias).map(|k| k.model_hosts).unwrap_or_default()
}

pub fn known(alias: &str) -> Option<Known> {
    all().into_iter().find(|k| k.alias == alias)
}
