//! A project's `tirith.toml`: its plan file, lane excludes, pipeline stages and extra rules.
use super::table::get;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize, Clone, Default)]
pub struct Stage {
    pub name: String,
    pub run: String,
    #[serde(default)]
    pub paths: Vec<String>,
    /// "deploy" marks a stage that publishes main: it runs after a merge, never as a check before it.
    #[serde(default)]
    pub kind: String,
}

#[derive(Deserialize, Clone, Default)]
pub struct RuleCfg {
    pub id: String,
    pub pattern: String,
    pub message: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub lanes_only: bool,
}

#[derive(Clone, Default)]
pub struct Manifest {
    pub roadmap: String,
    pub lane_exclude: Vec<String>,
    pub stages: Vec<Stage>,
    pub rules: Vec<RuleCfg>,
}

/// Every stage of every `[[pipelines]]`, in order.
fn stages(t: &toml::Table) -> Vec<Stage> {
    get::<Vec<toml::Table>>(t, &["pipelines"])
        .unwrap_or_default()
        .iter()
        .filter_map(|p| get::<Vec<Stage>>(p, &["stages"]))
        .flatten()
        .collect()
}

impl Manifest {
    pub fn load(repo: &Path) -> Manifest {
        let t: toml::Table = std::fs::read_to_string(repo.join("tirith.toml"))
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default();
        Manifest {
            roadmap: get(&t, &["roadmap"]).unwrap_or_else(|| "docs/TODO.md".into()),
            lane_exclude: get(&t, &["lane_exclude"]).unwrap_or_default(),
            stages: stages(&t),
            rules: get(&t, &["rules"]).unwrap_or_default(),
        }
    }
}
