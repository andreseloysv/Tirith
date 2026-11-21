//! The factory file (`factory/factory.toml` and its prompts): agents, roles, limits, plan words, card kinds and measures, read as data instead of Rust. Strict loading: an unknown key, a role on a missing agent or prompt, or an unknown measure makes a version invalid; the daemon keeps the last good one and reports the error. The repo's factory is embedded under the folder's as its fallback; `~/.tirith/config.toml` is laid over both key by key and wins.

use crate::config::get;
use serde::{de::DeserializeOwned, Deserialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::SystemTime;

const TEXT: &str = include_str!("../../../factory/factory.toml");
const SECTIONS: [&str; 7] = ["agents", "roles", "limits", "plan", "cards", "metrics", "routes"];
const MEASURES: [&str; 10] = ["merged", "median", "first_try", "idle", "stuck", "main_red", "failed", "owner", "interruptions", "needless_cards"];
/// The closed list of events the code emits, `;`-joined like the moves in config.rs; a route on anything else never fires, so it is invalid.
const EVENTS: &str = "owner.message;room.message;lane.asked;lane.failed;lane.not_started;lane.long;merge.blocked;merge.landed;main.red;main.green;plan.empty;deploy.done;deploy.failed;metric.red;review.done;schedule;card.opened;card.answered;card.closed;factory.invalid";
/// One `[[routes]]` entry read for one event: (to, mode, chat, phone, after_minutes).
pub type Route = (String, String, bool, bool, Option<u64>);

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
    /// This version's stamp: the main commit that last changed `factory/`, plus a short hash of what config.toml lays over it (`<commit>+<hash>`; `embedded` when no checkout is at hand).
    pub version: String,
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
        ("clippy", include_str!("../../../factory/prompts/clippy.md")),
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

/// An 8-hex-char hash of a text: short enough for a stamp, different whenever the text is.
pub fn short_hash(text: &str) -> String {
    crate::rooms::sealed::sha_hex(text.as_bytes())[..8].to_string()
}

/// The main commit that last changed the factory folder (its short hash), or `embedded` when no checkout is at hand.
fn commit_of(overlay: &toml::Table) -> String {
    let show = |rev: &str| -> Option<String> {
        let out = std::process::Command::new("git").arg("-C").arg(dir(overlay)?).args(["log", "-1", "--format=%h", rev, "--", "."]).output().ok()?;
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (out.status.success() && !text.is_empty()).then_some(text)
    };
    show("main").or_else(|| show("HEAD")).unwrap_or_else(|| "embedded".into())
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
    SECTIONS.into_iter().filter(|sec| *sec != "routes").filter_map(|sec| overlay.get(sec).map(|o| (sec, o))).for_each(|(sec, o)| merge(&mut table[sec], o));
    // Routes of config.toml (a bot's routines) stand next to the factory's, never instead of them.
    if let (Some(dst), Some(src)) = (table["routes"].as_array_mut(), overlay.get("routes").and_then(|r| r.as_array())) {
        dst.extend(src.iter().cloned());
    }
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
    let over: toml::Table = SECTIONS.into_iter().filter_map(|s| overlay.get(s).map(|v| (s.to_string(), v.clone()))).collect();
    let version = format!("{}+{}", commit_of(overlay), short_hash(&serde_json::to_string(&over).unwrap_or_default()));
    let f = Factory { raw, limits, prompts, version };
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
        let routes = t.get("routes").and_then(|r| r.as_array()).cloned().unwrap_or_default();
        let is_role = |n: &str| n.is_empty() || roles.contains_key(n);
        let bad = [
            t.keys().find(|k| !SECTIONS.contains(&k.as_str())).map(|k| format!("unknown section '{k}'")),
            unknown_in("agents", &["protocol", "model_flag", "effort_flag", "chat_args", "lane_args", "resume_args", "args", "command", "settings", "profile"]),
            unknown_in("roles", &["agent", "model", "effort", "fallback", "session", "scope", "extends", "access", "prompt", "idle_minutes", "max_per_day", "review_after_minutes", "runs_on", "name", "face"]),
            unknown_in("metrics", &["label", "target", "at_least"]),
            unknown_flat("plan", &["role_prefixes", "owner_wait", "waits_for"]),
            unknown_flat("cards", &["kinds"]),
            agents.iter().find(|(_, a)| !matches!(str_of(a, "protocol"), "" | "stream-json" | "exec")).map(|(n, a)| format!("agent '{n}' has unknown protocol '{}'", str_of(a, "protocol"))),
            roles.iter().find(|(_, r)| r.get("extends").is_some()).map(|(n, _)| format!("role '{n}' extends a missing role")),
            roles.iter().find_map(|(n, r)| r.get("agent").and_then(|x| x.as_str()).filter(|a| !agents.contains_key(*a)).map(|a| format!("role '{n}' runs on missing agent '{a}'"))),
            roles
                .iter()
                .find(|(_, r)| {
                    !matches!(str_of(r, "session"), "chat" | "turn" | "lane")
                        || !matches!(str_of(r, "scope"), "project" | "global" | "own")
                        || !matches!(str_of(r, "access"), "" | "read" | "write")
                        || !matches!(str_of(r, "runs_on"), "" | "local")
                })
                .map(|(n, r)| format!("role '{n}' has unknown session '{}', scope '{}', access '{}' or runs_on '{}'", str_of(r, "session"), str_of(r, "scope"), str_of(r, "access"), str_of(r, "runs_on"))),
            roles
                .iter()
                .find_map(|(n, r)| r.get("prompt").and_then(|p| p.as_array()).into_iter().flatten().filter_map(|p| p.as_str()).find(|p| !self.prompts.contains_key(&stem(p))).map(|p| format!("role '{n}' names missing prompt '{p}'"))),
            sub("metrics").and_then(|m| m.keys().find(|id| !MEASURES.contains(&id.as_str())).map(|id| format!("unknown measure '{id}'"))),
            routes.iter().find_map(|r| {
                r.as_table().and_then(|r| r.keys().find(|k| !["on", "to", "mode", "chat", "phone", "after_minutes", "then", "days", "at", "task", "title", "id", "enabled"].contains(&k.as_str())).map(|k| format!("unknown key routes.{k}")))
            }),
            routes
                .iter()
                .find(|r| !EVENTS.split(';').any(|e| e == str_of(r, "on")) || !is_role(str_of(r, "to")) || !is_role(str_of(r, "then")) || !matches!(str_of(r, "mode"), "" | "turn" | "note"))
                .map(|r| format!("route on '{}' names a missing event, role or mode ('{}', '{}')", str_of(r, "on"), str_of(r, "to"), str_of(r, "mode"))),
        ]
        .into_iter()
        .flatten()
        .next();
        bad.map_or(Ok(()), |e| Err(format!("factory: {e}")))
    }

    pub fn agent<T: DeserializeOwned>(&self, alias: &str, key: &str) -> Option<T> {
        let alias = if self.raw["agents"].get(alias).is_some() { alias } else { "default" };
        get(&self.raw, &["agents", alias, key])
    }

    pub fn command(&self, alias: &str) -> Option<String> {
        get(&self.raw, &["agents", alias, "command"])
    }

    pub fn commands(&self) -> Vec<(String, String)> {
        self.raw["agents"].as_table().into_iter().flatten().filter_map(|(n, a)| a.get("command").and_then(|c| c.as_str()).map(|c| (n.clone(), c.to_string()))).collect()
    }

    /// One key of a role, by its factory name.
    pub fn role<T: DeserializeOwned>(&self, name: &str, key: &str) -> Option<T> {
        get(&self.raw, &["roles", name, key])
    }

    /// Every role of the factory, in name order.
    pub fn roles(&self) -> Vec<String> {
        self.raw["roles"].as_table().into_iter().flatten().map(|(n, _)| n.clone()).collect()
    }

    /// The first role (in name order) with this `session` and, when given, this `scope`: how code finds who runs a lane, a project's chat or a turn without naming a role.
    pub fn find_role(&self, session: &str, scope: Option<&str>) -> Option<String> {
        self.roles().into_iter().find(|n| self.role::<String>(n, "session").as_deref() == Some(session) && scope.is_none_or(|s| self.role::<String>(n, "scope").as_deref() == Some(s)))
    }

    pub fn metric(&self, id: &str) -> (String, Option<f64>, bool) {
        (get(&self.raw, &["metrics", id, "label"]).unwrap_or_default(), get(&self.raw, &["metrics", id, "target"]), get(&self.raw, &["metrics", id, "at_least"]).unwrap_or(false))
    }

    pub fn words(&self, sec: &str, key: &str) -> Vec<String> {
        get(&self.raw, &[sec, key]).unwrap_or_default()
    }

    /// Who hears `on`: (to, mode, chat, phone, after_minutes) of every `[[routes]]` on it.
    pub fn routes_on(&self, on: &str) -> Vec<Route> {
        let rows = self.raw.get("routes").and_then(|r| r.as_array()).into_iter().flatten().filter(|r| str_of(r, "on") == on);
        let b = |r: &toml::Value, k: &str| r.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
        rows.map(|r| (str_of(r, "to").into(), str_of(r, "mode").into(), b(r, "chat"), b(r, "phone"), r.get("after_minutes").and_then(|v| v.as_integer()).and_then(|n| u64::try_from(n).ok()))).collect()
    }

    pub fn prompt(&self, name: &str) -> String {
        self.prompts.get(name).cloned().unwrap_or_else(|| embedded_prompts().get(name).cloned().unwrap_or_default())
    }

    /// A short hash of the setting `role` runs with: its resolved table (agent, model, effort, ...), its agent's table, its prompt texts and `extra` (the page's model override for it, when the caller has one). An unknown role has none (an empty stamp).
    pub fn role_hash(&self, role: &str, extra: &str) -> String {
        let Some(r) = self.raw.get("roles").and_then(|s| s.get(role)) else { return String::new() };
        let json = |v: Option<&toml::Value>| serde_json::to_string(&v).unwrap_or_default();
        short_hash(&format!("{}|{}|{}|{extra}", json(Some(r)), json(self.raw.get("agents").and_then(|s| s.get(str_of(r, "agent")))), self.prompts_of(role)))
    }

    /// A role's prompt files, one after another, with `{name}` its factory name (a bot's, from config.toml).
    pub fn prompts_of(&self, role: &str) -> String {
        self.role::<Vec<String>>(role, "prompt").unwrap_or_default().iter().map(|p| self.prompt(&stem(p))).collect::<Vec<_>>().join("\n").replace("{name}", &self.role::<String>(role, "name").unwrap_or_else(|| role.to_string()))
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
    let prompts = |dir: &Path| -> BTreeMap<String, String> {
        std::fs::read_dir(dir.join("prompts"))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "md"))
            .filter_map(|e| Some((stem(&e.path().to_string_lossy()), std::fs::read_to_string(e.path()).ok()?)))
            .collect()
    };
    let attempt = match dir(overlay) {
        Some(d) => std::fs::read_to_string(d.join("factory.toml")).map_err(|e| format!("factory: {}: {e}", d.display())).and_then(|text| parse(&text, prompts(&d), overlay)),
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
