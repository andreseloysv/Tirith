//! The config of one OpenBot home, as read from its config.toml.
use super::project::Project;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct Config {
    /// The whole file (with yesterday's sections already under their new names), for modules that read their own sections (`[network]`, `[messenger]`, `[guard]`, ...).
    pub raw: toml::Table,
    pub projects: Vec<Project>,
    pub launch: Vec<String>,
    pub port: u16,
    pub push: bool,
    /// `[autopilot] enabled`: the default of each project's autopilot switch.
    pub autopilot: bool,
    /// The factory with this file laid over it (agents, roles, limits, plan words, cards, measures, prompts).
    pub factory: Arc<crate::factory::Factory>,
    /// Why this file or the factory is not as written (a parse error, an unknown key, an invalid factory): shown on the page and told to the manager of no project, never silently ignored.
    pub error: Option<String>,
}

impl Config {
    pub fn project(&self, id: &str) -> Option<&Project> {
        self.projects.iter().find(|p| p.id == id)
    }
}
