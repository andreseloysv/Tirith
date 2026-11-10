//! `~/.tirith/config.toml` (projects, agent aliases, limits) and each project's `tirith.toml` (plan file,
//! checks, lane excludes, extra rules). Unknown keys are ignored, so older files keep working.

use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub fn expand(p: &str) -> PathBuf {
    p.strip_prefix("~/").map_or_else(|| PathBuf::from(p), |rest| home_dir().join(rest))
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

pub fn tirith_home() -> PathBuf {
    std::env::var_os("TIRITH_HOME").map(PathBuf::from).unwrap_or_else(|| home_dir().join(".tirith"))
}

#[derive(Deserialize, Clone, Default)]
pub struct Project {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub color: Option<String>,
}

impl Project {
    pub fn repo(&self) -> PathBuf {
        let p = expand(&self.repo);
        p.canonicalize().unwrap_or(p)
    }
    /// Where the lanes' worktrees live: `<tirith home>/lanes/<id>`, out of the owner's own folders.
    pub fn lanes_dir(&self) -> PathBuf {
        let home = tirith_home();
        home.canonicalize().unwrap_or(home).join("lanes").join(&self.id)
    }
}

#[derive(Deserialize, Clone, Default)]
pub struct Agent {
    pub command: String,
    #[serde(default)]
    pub settings: Option<String>,
    /// Arguments of the manager's streaming chat session.
    #[serde(default)]
    pub chat_args: Option<Vec<String>>,
    /// Arguments of a lane run; `{brief}` and `{workspace}` are filled in.
    #[serde(default)]
    pub args: Option<Vec<String>>,
}

#[derive(Clone, Default)]
pub struct Config {
    /// The whole file, for modules that read their own sections (`[network]`, `[messenger]`, `[guard]`, ...).
    pub raw: toml::Table,
    pub projects: Vec<Project>,
    pub agents: BTreeMap<String, Agent>,
    pub launch: Vec<String>,
    /// `[models.workers] agent`; the models module reads the rest of `[models.*]`.
    pub workers_agent: Option<String>,
    pub port: u16,
    pub max_parallel: usize,
    pub max_load: f64,
    pub inactivity_min: u64,
    pub push: bool,
    pub stage_timeout_min: u64,
    pub plan_commit_s: u64,
    /// `[autopilot] enabled`: the default of each project's autopilot switch.
    pub autopilot: bool,
}

pub fn get<T: DeserializeOwned>(t: &toml::Table, path: &[&str]) -> Option<T> {
    path[1..].iter().try_fold(t.get(path[0])?, |v, key| v.get(key))?.clone().try_into().ok()
}

impl Config {
    pub fn load() -> Config {
        let t: toml::Table = std::fs::read_to_string(tirith_home().join("config.toml")).ok().and_then(|s| s.parse().ok()).unwrap_or_default();
        Config {
            raw: t.clone(),
            projects: get(&t, &["projects"]).unwrap_or_default(),
            agents: get(&t, &["agents"]).unwrap_or_default(),
            launch: get(&t, &["launch", "command"]).unwrap_or_default(),
            workers_agent: get(&t, &["models", "workers", "agent"]),
            port: std::env::var("TIRITH_PORT").ok().and_then(|p| p.parse().ok()).or_else(|| get(&t, &["server", "port"])).unwrap_or(7717),
            max_parallel: get(&t, &["lanes", "max_parallel"]).or_else(|| get(&t, &["autopilot", "max_parallel"])).unwrap_or(3),
            max_load: get(&t, &["lanes", "max_load"]).unwrap_or(2.0),
            inactivity_min: get(&t, &["lanes", "inactivity_minutes"]).unwrap_or(20),
            push: get(&t, &["merge", "push"]).unwrap_or(true),
            stage_timeout_min: get(&t, &["merge", "stage_timeout_minutes"]).unwrap_or(30),
            plan_commit_s: get(&t, &["plan", "commit_seconds"]).unwrap_or(120),
            autopilot: get(&t, &["autopilot", "enabled"]).unwrap_or(true),
        }
    }

    pub fn project(&self, id: &str) -> Option<&Project> {
        self.projects.iter().find(|p| p.id == id)
    }

    /// The project a folder belongs to: inside its repo or inside its lanes folder.
    pub fn project_for(&self, cwd: &str) -> Option<&Project> {
        let cwd = Path::new(cwd);
        self.projects.iter().filter(|p| !p.repo.is_empty()).find(|p| cwd.starts_with(p.repo()) || cwd.starts_with(p.lanes_dir()))
    }
}

#[derive(Deserialize, Clone, Default)]
pub struct Stage {
    pub name: String,
    pub run: String,
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Deserialize, Clone, Default)]
pub struct Pipeline {
    #[serde(default)]
    pub stages: Vec<Stage>,
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

impl Manifest {
    pub fn load(repo: &Path) -> Manifest {
        let t: toml::Table = std::fs::read_to_string(repo.join("tirith.toml")).ok().and_then(|s| s.parse().ok()).unwrap_or_default();
        Manifest {
            roadmap: get(&t, &["roadmap"]).unwrap_or_else(|| "docs/TODO.md".into()),
            lane_exclude: get(&t, &["lane_exclude"]).unwrap_or_default(),
            stages: get::<Vec<Pipeline>>(&t, &["pipelines"]).unwrap_or_default().into_iter().flat_map(|p| p.stages).collect(),
            rules: get(&t, &["rules"]).unwrap_or_default(),
        }
    }
}
