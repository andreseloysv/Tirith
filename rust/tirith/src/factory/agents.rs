//! Reading a factory's agents: their keys and their commands.
use super::loaded::Factory;
use crate::config::get;
use serde::de::DeserializeOwned;

impl Factory {
    /// One key of an agent, by its alias; an unknown alias reads the `default` agent.
    pub fn agent<T: DeserializeOwned>(&self, alias: &str, key: &str) -> Option<T> {
        let alias = if self.raw["agents"].get(alias).is_some() {
            alias
        } else {
            "default"
        };
        get(&self.raw, &["agents", alias, key])
    }

    pub fn command(&self, alias: &str) -> Option<String> {
        get(&self.raw, &["agents", alias, "command"])
    }

    /// Every agent with a command: (alias, command).
    pub fn commands(&self) -> Vec<(String, String)> {
        self.raw["agents"]
            .as_table()
            .into_iter()
            .flatten()
            .filter_map(|(n, a)| {
                a.get("command")
                    .and_then(|c| c.as_str())
                    .map(|c| (n.clone(), c.to_string()))
            })
            .collect()
    }
}
