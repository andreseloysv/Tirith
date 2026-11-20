//! `~/.tirith/config.toml` (projects, agent aliases, this machine's overlay on the factory) and each project's `tirith.toml` (plan file, checks, lane excludes, extra rules). Unknown keys are an error the owner sees, never silently ignored; yesterday's sections are read in memory under the factory's names, the file itself untouched.

use serde::{de::DeserializeOwned, Deserialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

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

pub fn get<T: DeserializeOwned>(t: &toml::Table, path: &[&str]) -> Option<T> {
    path[1..].iter().try_fold(t.get(path[0])?, |v, key| v.get(key))?.clone().try_into().ok()
}

/// Yesterday's key -> today's: `[models.*]` and `[ceo]` become `[roles.*]` and `[metrics.*]`, the `[lanes]`/`[merge]`/`[plan]` limits become `[limits]`.
const MOVES: &str = "models.manager>roles.manager;models.workers>roles.worker;models.ceo>roles.reviewer;ceo.agent>roles.reviewer.agent;ceo.model>roles.reviewer.model;ceo.effort>roles.reviewer.effort;ceo.fallback>roles.reviewer.fallback;ceo.max_per_day>roles.reviewer.max_per_day;ceo.review_after_minutes>roles.reviewer.review_after_minutes;ceo.targets.merged_per_day>metrics.merged.target;ceo.targets.median_merge_minutes>metrics.median.target;ceo.targets.first_try_pct>metrics.first_try.target;ceo.targets.idle_minutes>metrics.idle.target;ceo.targets.stuck>metrics.stuck.target;ceo.targets.main_red>metrics.main_red.target;lanes.max_load>limits.max_load;lanes.inactivity_minutes>limits.inactivity_minutes;lanes.long_running_minutes>limits.long_running_minutes;lanes.max_parallel>limits.max_parallel;autopilot.max_parallel>limits.max_parallel;merge.stage_timeout_minutes>limits.stage_timeout_minutes;plan.commit_seconds>limits.plan_commit_seconds";

fn take(t: &mut toml::Table, path: &[&str]) -> Option<toml::Value> {
    let (first, rest) = path.split_first()?;
    if rest.is_empty() {
        return t.remove(*first);
    }
    let child = t.get_mut(*first)?.as_table_mut()?;
    let v = take(child, rest)?;
    let _ = child.is_empty().then(|| t.remove(*first));
    Some(v)
}

/// Puts `v` (yesterday's value) at `path`, under anything already written there under its new name.
fn put(t: &mut toml::Table, path: &[&str], v: toml::Value) {
    if path.len() == 1 {
        let e = t.entry(path[0]).or_insert_with(|| v.clone());
        let old = std::mem::replace(e, v);
        crate::factory::merge(e, &old);
    } else if let Some(child) = t.entry(path[0]).or_insert_with(|| toml::Value::Table(Default::default())).as_table_mut() {
        put(child, &path[1..], v);
    }
}

/// Moves yesterday's sections under their new names (a key already written under its new name wins); true when anything moved.
pub fn migrate(t: &mut toml::Table) -> bool {
    let mut moved = false;
    for (from, to) in MOVES.split(';').filter_map(|m| m.split_once('>')) {
        if let Some(v) = take(t, &from.split('.').collect::<Vec<_>>()) {
            put(t, &to.split('.').collect::<Vec<_>>(), v);
            moved = true;
        }
    }
    moved
}

impl Config {
    pub fn load() -> Config {
        let text = std::fs::read_to_string(tirith_home().join("config.toml")).unwrap_or_default();
        let (mut t, parsed) = match text.parse::<toml::Table>() {
            Ok(t) => (t, None),
            Err(_) if text.trim().is_empty() => (Default::default(), None),
            Err(e) => (Default::default(), Some(format!("config.toml does not parse ({e}); the defaults run instead"))),
        };
        migrate(&mut t);
        let factory = crate::factory::load(&t);
        // Keys nothing reads any more, named for the owner: an unknown section, or a leftover `[models.*]` role (`choices`/`efforts` lists stay). The factory-shaped sections are checked by the factory loader itself.
        const KNOWN: [&str; 20] = ["projects", "agents", "launch", "models", "server", "merge", "autopilot", "app", "network", "messenger", "owner", "rooms", "guard", "factory", "roles", "limits", "plan", "cards", "metrics", "routes"];
        let mut unknown: Vec<String> = t.keys().filter(|k| !KNOWN.contains(&k.as_str())).cloned().collect();
        if let Some(m) = t.get("models").and_then(|v| v.as_table()) {
            unknown.extend(m.keys().filter(|k| !matches!(k.as_str(), "choices" | "efforts")).map(|k| format!("models.{k}")));
        }
        let error = [parsed, crate::factory::error().map(|e| format!("factory.invalid: {e} (the last good factory keeps running)")), (!unknown.is_empty()).then(|| format!("config.toml has keys nothing reads: {}", unknown.join(", ")))]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        Config {
            raw: t.clone(),
            projects: get(&t, &["projects"]).unwrap_or_default(),
            launch: get(&t, &["launch", "command"]).unwrap_or_default(),
            port: std::env::var("TIRITH_PORT").ok().and_then(|p| p.parse().ok()).or_else(|| get(&t, &["server", "port"])).unwrap_or(7717),
            push: get(&t, &["merge", "push"]).unwrap_or(true),
            autopilot: get(&t, &["autopilot", "enabled"]).unwrap_or(true),
            factory,
            error: (!error.is_empty()).then(|| error.join("; ")),
        }
    }

    pub fn project(&self, id: &str) -> Option<&Project> {
        self.projects.iter().find(|p| p.id == id)
    }
}

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

impl Manifest {
    pub fn load(repo: &Path) -> Manifest {
        let t: toml::Table = std::fs::read_to_string(repo.join("tirith.toml")).ok().and_then(|s| s.parse().ok()).unwrap_or_default();
        Manifest {
            roadmap: get(&t, &["roadmap"]).unwrap_or_else(|| "docs/TODO.md".into()),
            lane_exclude: get(&t, &["lane_exclude"]).unwrap_or_default(),
            stages: get::<Vec<toml::Table>>(&t, &["pipelines"]).unwrap_or_default().iter().filter_map(|p| get::<Vec<Stage>>(p, &["stages"])).flatten().collect(),
            rules: get(&t, &["rules"]).unwrap_or_default(),
        }
    }
}
