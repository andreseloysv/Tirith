//! The factory file (`factory/factory.toml` and its prompts): agents, roles, limits, plan words, card kinds and measures, read as data instead of Rust. Strict loading: an unknown key, a role on a missing agent or prompt, or an unknown measure makes a version invalid; the daemon keeps the last good one and reports the error. The repo's factory is embedded under the folder's as its fallback; `~/.tirith/config.toml` is laid over both key by key and wins.

use crate::config::get;
use serde::{de::DeserializeOwned, Deserialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::SystemTime;

const TEXT: &str = include_str!("../../../factory/factory.toml");
const SECTIONS: [&str; 6] = ["agents", "roles", "limits", "plan", "cards", "metrics"];
const MEASURES: [&str; 8] = ["merged", "median", "first_try", "idle", "stuck", "main_red", "failed", "owner"];

#[derive(Deserialize, Clone, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    pub max_load: f64,
    pub inactivity_minutes: u64,
    pub long_running_minutes: u64, // a lane running this long is reported once to its manager (0 turns it off)
    pub stage_timeout_minutes: u64,
    pub merge_train: usize,
    pub merge_backlog: usize,
    pub low_disk_gb: f64,
    pub plan_commit_seconds: u64,
    pub batch_seconds: u64,
    pub agent_messages_in_a_row: u32,
    pub max_parallel: Option<usize>, // a cap on lanes at once; none unless config.toml sets one
}

/// One loaded factory version: the merged table (embedded, then the folder's file, then config.toml), its limits read from it, and its prompts by file stem.
#[derive(Clone)]
pub struct Factory {
    raw: toml::Table,
    pub limits: Limits,
    prompts: BTreeMap<String, String>,
}

impl Default for Factory {
    // The embedded factory: what runs when no factory folder is found.
    fn default() -> Self {
        parse(TEXT, embedded_prompts(), &toml::Table::new()).expect("the embedded factory parses")
    }
}

fn embedded_prompts() -> BTreeMap<String, String> {
    [
        ("manager", include_str!("../../../factory/prompts/manager.md")),
        ("one-project", include_str!("../../../factory/prompts/one-project.md")),
        ("no-project", include_str!("../../../factory/prompts/no-project.md")),
        ("brief", include_str!("../../../factory/prompts/brief.md")),
        ("reviewer", include_str!("../../../factory/prompts/reviewer.md")),
        ("bot", include_str!("../../../factory/prompts/bot.md")),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

/// Lays `src` over `dst` key by key: tables merge, anything else replaces.
pub(crate) fn merge(dst: &mut toml::Value, src: &toml::Value) {
    match (dst.as_table_mut(), src.as_table()) {
        (Some(d), Some(s)) => s.iter().for_each(|(k, v)| merge(d.entry(k.clone()).or_insert_with(|| toml::Value::Table(Default::default())), v)),
        _ => *dst = src.clone(),
    }
}

fn stem(path: &str) -> String {
    Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

fn str_of<'a>(v: &'a toml::Value, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("")
}

/// A role's table with what it extends under it, all the way up; a missing parent or a cycle keeps `extends` and fails validation.
fn resolved(roles: &toml::Table, name: &str, up: usize) -> toml::Value {
    let mut own = roles.get(name).cloned().unwrap_or(toml::Value::Table(Default::default()));
    let Some(parent) = own.get("extends").and_then(|e| e.as_str()).filter(|_| up < 8).map(String::from) else { return own };
    own.as_table_mut().map(|t| t.remove("extends"));
    let mut base = resolved(roles, &parent, up + 1);
    let solved = roles.contains_key(&parent) && base.get("extends").is_none();
    merge(&mut base, &own);
    if !solved {
        base.as_table_mut().map(|t| t.insert("extends".into(), parent.into())); // stays, so validation names it
    }
    base
}

/// Parses and validates one factory version: the embedded text under the file's, its prompts (by file stem) and config.toml laid over both.
pub fn parse(text: &str, prompts: BTreeMap<String, String>, overlay: &toml::Table) -> Result<Factory, String> {
    let mut table = toml::Value::Table(TEXT.parse().map_err(|e| format!("factory.toml: {e}"))?);
    merge(&mut table, &toml::Value::Table(text.parse().map_err(|e| format!("factory.toml: {e}"))?));
    SECTIONS.into_iter().filter_map(|sec| overlay.get(sec).map(|o| (sec, o))).for_each(|(sec, o)| merge(&mut table[sec], o));
    let roles = table["roles"].as_table().cloned().unwrap_or_default();
    roles.keys().cloned().collect::<Vec<_>>().iter().for_each(|name| table["roles"][name] = resolved(&roles, name, 0));
    // The `default` agent under every other (never its chat args for an exec agent), and the codex command needs no config line, as before.
    let default = table["agents"]["default"].clone();
    if let Some(agents) = table["agents"].as_table_mut() {
        for a in agents.iter_mut().filter(|(n, _)| *n != "default").map(|(_, a)| a) {
            let mut under = default.clone();
            let _ = (a.get("protocol").and_then(|p| p.as_str()) == Some("exec")).then(|| under.as_table_mut().map(|t| t.remove("chat_args")));
            merge(&mut under, a);
            *a = under;
        }
    }
    if let Some(c) = table["agents"].as_table_mut().and_then(|a| a.get_mut("codex")).and_then(|c| c.as_table_mut()) {
        c.entry("command").or_insert_with(|| "codex".into());
    }
    let raw = table.as_table().cloned().unwrap_or_default();
    let limits: Limits = raw.get("limits").cloned().map(|v| v.try_into()).transpose().map_err(|e| format!("factory: {e}"))?.unwrap_or_default();
    let f = Factory { raw, limits, prompts };
    f.validate()?;
    Ok(f)
}

impl Factory {
    fn validate(&self) -> Result<(), String> {
        let t = &self.raw;
        let sub = |sec: &str| t.get(sec).and_then(|v| v.as_table());
        let unknown_in = |sec: &str, allowed: &[&str]| sub(sec).and_then(|m| m.iter().find_map(|(n, s)| s.as_table().and_then(|s| s.keys().find(|k| !allowed.contains(&k.as_str())).map(|k| format!("unknown key {sec}.{n}.{k}")))));
        let unknown_flat = |sec: &str, allowed: &[&str]| sub(sec).and_then(|m| m.keys().find(|k| !allowed.contains(&k.as_str())).map(|k| format!("unknown key {sec}.{k}")));
        let (agents, roles) = (sub("agents").cloned().unwrap_or_default(), sub("roles").cloned().unwrap_or_default());
        let mut bad: Option<String> = None;
        let mut chk = |o: Option<String>| {
            if bad.is_none() {
                bad = o
            }
        };
        chk(t.keys().find(|k| !SECTIONS.contains(&k.as_str())).map(|k| format!("unknown section '{k}'")));
        chk(unknown_in("agents", &["protocol", "model_flag", "effort_flag", "chat_args", "lane_args", "resume_args", "args", "command", "settings", "profile"]));
        chk(unknown_in("roles", &["agent", "model", "effort", "fallback", "session", "scope", "extends", "access", "prompt", "idle_minutes", "max_per_day", "review_after_minutes", "runs_on", "name"]));
        chk(unknown_in("metrics", &["label", "target", "at_least"]));
        chk(unknown_flat("plan", &["role_prefixes", "owner_wait", "waits_for"]));
        chk(unknown_flat("cards", &["kinds"]));
        chk(agents.iter().find(|(_, a)| !matches!(str_of(a, "protocol"), "" | "stream-json" | "exec")).map(|(n, a)| format!("agent '{n}' has unknown protocol '{}'", str_of(a, "protocol"))));
        chk(roles.iter().find(|(_, r)| r.get("extends").is_some()).map(|(n, _)| format!("role '{n}' extends a missing role")));
        chk(roles.iter().find_map(|(n, r)| r.get("agent").and_then(|x| x.as_str()).filter(|a| !agents.contains_key(*a)).map(|a| format!("role '{n}' runs on missing agent '{a}'"))));
        chk(roles
            .iter()
            .find(|(_, r)| !matches!(str_of(r, "session"), "" | "chat" | "turn" | "lane") || !matches!(str_of(r, "scope"), "" | "project" | "global" | "own"))
            .map(|(n, r)| format!("role '{n}' has unknown session '{}' or scope '{}'", str_of(r, "session"), str_of(r, "scope"))));
        chk(roles
            .iter()
            .find_map(|(n, r)| r.get("prompt").and_then(|p| p.as_array()).into_iter().flatten().filter_map(|p| p.as_str()).find(|p| !self.prompts.contains_key(&stem(p))).map(|p| format!("role '{n}' names missing prompt '{p}'"))));
        chk(sub("metrics").and_then(|m| m.keys().find(|id| !MEASURES.contains(&id.as_str())).map(|id| format!("unknown measure '{id}'"))));
        match bad {
            Some(e) => Err(format!("factory: {e}")),
            None => Ok(()),
        }
    }

    fn v<T: DeserializeOwned>(&self, path: &[&str]) -> Option<T> {
        get(&self.raw, path)
    }

    pub fn agent<T: DeserializeOwned>(&self, alias: &str, key: &str) -> Option<T> {
        let alias = if self.raw["agents"].get(alias).is_some() { alias } else { "default" };
        self.v(&["agents", alias, key])
    }

    pub fn command(&self, alias: &str) -> Option<String> {
        self.v(&["agents", alias, "command"])
    }

    pub fn commands(&self) -> Vec<(String, String)> {
        self.raw["agents"].as_table().into_iter().flatten().filter_map(|(n, a)| a.get("command").and_then(|c| c.as_str()).map(|c| (n.clone(), c.to_string()))).collect()
    }

    /// One key of a role, by the name the code uses today (`workers`, `ceo`) or its factory name (`worker`, `reviewer`).
    pub fn role<T: DeserializeOwned>(&self, name: &str, key: &str) -> Option<T> {
        self.v(&[
            "roles",
            match name {
                "workers" => "worker",
                "ceo" => "reviewer",
                n => n,
            },
            key,
        ])
    }

    pub fn metric(&self, id: &str) -> (String, Option<f64>, bool) {
        (self.v(&["metrics", id, "label"]).unwrap_or_default(), self.v(&["metrics", id, "target"]), self.v(&["metrics", id, "at_least"]).unwrap_or(false))
    }

    pub fn words(&self, sec: &str, key: &str) -> Vec<String> {
        self.v(&[sec, key]).unwrap_or_default()
    }

    pub fn prompt(&self, name: &str) -> String {
        self.prompts.get(name).cloned().unwrap_or_else(|| embedded_prompts().get(name).cloned().unwrap_or_default())
    }

    /// A role's prompt files, one after another.
    pub fn prompts_of(&self, role: &str) -> String {
        self.role::<Vec<String>>(role, "prompt").unwrap_or_default().iter().map(|p| self.prompt(&stem(p))).collect::<Vec<_>>().join("\n")
    }
}

fn dir(overlay: &toml::Table) -> Option<PathBuf> {
    let here = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../factory"));
    overlay.get("factory").and_then(|v| v.as_str()).map(crate::config::expand).or_else(|| here.join("factory.toml").is_file().then_some(here))
}

pub fn stamp(overlay: &toml::Table) -> Option<SystemTime> {
    let dir = dir(overlay)?;
    let m = |p: &Path| std::fs::metadata(p).and_then(|x| x.modified()).ok();
    std::fs::read_dir(dir.join("prompts")).into_iter().flatten().flatten().filter_map(|e| m(&e.path())).chain(m(&dir.join("factory.toml"))).max()
}

fn read_prompts(dir: &Path) -> BTreeMap<String, String> {
    std::fs::read_dir(dir.join("prompts"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "md"))
        .filter_map(|e| Some((stem(&e.path().to_string_lossy()), std::fs::read_to_string(e.path()).ok()?)))
        .collect()
}

static CURRENT: RwLock<Option<Arc<Factory>>> = RwLock::new(None);
static ERROR: Mutex<Option<String>> = Mutex::new(None);

pub fn current() -> Arc<Factory> {
    static EMBEDDED: OnceLock<Arc<Factory>> = OnceLock::new();
    CURRENT.read().unwrap().clone().unwrap_or_else(|| EMBEDDED.get_or_init(|| Arc::new(Factory::default())).clone())
}

/// Why the newest factory version did not load; the last good one keeps running.
pub fn error() -> Option<String> {
    ERROR.lock().unwrap().clone()
}

/// (Re)loads the factory under config.toml's overlay; a bad version keeps the last good one and is remembered in `error()`.
pub fn load(overlay: &toml::Table) -> Arc<Factory> {
    let attempt = match dir(overlay) {
        Some(d) => std::fs::read_to_string(d.join("factory.toml")).map_err(|e| format!("factory: {}: {e}", d.display())).and_then(|text| parse(&text, read_prompts(&d), overlay)),
        None => parse(TEXT, embedded_prompts(), overlay),
    };
    let f = match attempt {
        Ok(f) => {
            *ERROR.lock().unwrap() = None;
            Arc::new(f)
        }
        Err(e) => {
            *ERROR.lock().unwrap() = Some(e);
            return current();
        }
    };
    *CURRENT.write().unwrap() = Some(f.clone());
    f
}

#[cfg(test)]
#[path = "../checks/factory.rs"]
mod tests;
