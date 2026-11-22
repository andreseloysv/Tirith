//! One loaded factory version.
use super::embedded::{embedded_prompts, TEXT};
use super::lab::Lab;
use super::limits::Limits;
use std::collections::BTreeMap;

/// One loaded factory version: the merged table (embedded, then the folder's file, then config.toml, extends resolved), its limits read from it, and its prompts by file stem. `raw` is that table, for a reader that walks it as data (the bots).
#[derive(Clone)]
pub struct Factory {
    pub raw: toml::Table,
    pub limits: Limits,
    /// Its `[lab]`: the rule experiments are decided by.
    pub lab: Lab,
    pub(super) prompts: BTreeMap<String, String>,
    /// config.toml's `[[connectors]]`, checked with the grants under the roles.
    pub connectors: Vec<crate::connectors::Connector>,
    /// This version's stamp: the main commit that last changed `factory/`, plus a short hash of what config.toml lays over it (`<commit>+<hash>`; `embedded` when no checkout is at hand).
    pub version: String,
}

impl Default for Factory {
    // The embedded factory: what runs when no factory folder is found.
    fn default() -> Self {
        super::parse(TEXT, embedded_prompts(), &toml::Table::new())
            .expect("the embedded factory parses")
    }
}
