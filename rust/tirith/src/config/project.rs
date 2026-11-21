//! One `[[projects]]` entry of config.toml.
use super::paths::{expand, tirith_home};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Clone, Default)]
pub struct Project {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub color: Option<String>,
    /// How long this project's chat lives, in the owner's own words ("24h", "2 días"; empty is forever) — see chat.rs.
    #[serde(default)]
    pub chat_keep: String,
}

impl Project {
    pub fn repo(&self) -> PathBuf {
        let p = expand(&self.repo);
        p.canonicalize().unwrap_or(p)
    }
    /// Where the lanes' worktrees live: `<tirith home>/lanes/<id>`, out of the owner's own folders.
    pub fn lanes_dir(&self) -> PathBuf {
        let home = tirith_home();
        home.canonicalize()
            .unwrap_or(home)
            .join("lanes")
            .join(&self.id)
    }
}
