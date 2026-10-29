//! TIRITH_HOME layout and `config.toml` loading (`home.py`).
//!
//! Loading never fails: a missing or broken file gives the defaults, and a value of the wrong type
//! falls back to its default, exactly as the Python loader does.

use crate::pytext::{expanduser, get_str, str_of, truthy};
use std::fs;
use std::path::{Path, PathBuf};
use toml::{Table, Value};

pub const DEFAULT_MODEL: &str = "muse-spark-1.3";
pub const DEFAULT_BASE_URL: &str = "https://api.meta.ai/v1";
pub const DEFAULT_MANAGER_NAME: &str = "Aldric of the Tower";
pub const DEFAULT_MANAGER_TITLE: &str = "The Manager";
pub const DEFAULT_COLOR: &str = "#888888";

/// The Tirith home: the explicit argument, then `TIRITH_HOME`, then `~/.tirith`.
pub fn tirith_home(explicit: Option<&Path>) -> PathBuf {
    if let Some(path) = explicit.filter(|p| !p.as_os_str().is_empty()) {
        return expanduser(&path.to_string_lossy());
    }
    if let Some(env) = std::env::var_os("TIRITH_HOME").filter(|v| !v.is_empty()) {
        return expanduser(&env.to_string_lossy());
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".tirith")
}

/// Create the home layout (idempotent) and return the home path.
pub fn ensure_home(home: &Path) -> std::io::Result<PathBuf> {
    for sub in ["lanes", "logs", "brain"] {
        fs::create_dir_all(home.join(sub))?;
    }
    Ok(home.to_path_buf())
}

/// One project row from `config.toml`.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectConfig {
    pub id: String,
    pub name: String,
    pub repo: Option<PathBuf>,
    pub color: String,
    pub guard_words: Option<Vec<String>>,
    pub architecture: Option<Table>,
    /// `guard = false` opts the project out of the commit guard.
    pub guard: bool,
    pub cargo_target_dir: Option<PathBuf>,
}

/// One `[[rooms]]` row: an agentsmessenger.com room Tirith watches.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RoomConfig {
    pub id: String,
    pub project: Option<String>,
    /// Room participant name -> Tirith lane id.
    pub aliases: Vec<(String, String)>,
    pub title: Option<String>,
}

/// Parsed `config.toml` (a missing file means the empty config).
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub projects: Vec<ProjectConfig>,
    pub brain_model: String,
    pub brain_base_url: String,
    pub brain_enabled: bool,
    pub brain_reasoning_effort: String,
    pub manager_name: String,
    pub manager_title: String,
    pub guard_words: Option<Vec<String>>,
    pub rooms: Vec<RoomConfig>,
    pub owner_names: Vec<String>,
    pub rooms_interval_s: f64,
    /// `[agents.<alias>]` tables, in file order.
    pub agents: Vec<(String, Table)>,
    pub gates_enabled: bool,
    pub github_token_env: Option<String>,
    pub github_agent_logins: Vec<String>,
    pub app_tirith: Option<String>,
    pub app_editor: Option<String>,
    /// `[launch] command`: a string or a list of strings.
    pub launch_command: Option<Value>,
    pub dispatch_enabled: bool,
    pub dispatch_auto_launch: bool,
    pub autopilot_enabled: bool,
    pub autopilot_max_parallel: i64,
    pub review_auto_merge: bool,
    pub disk_min_free_gb: f64,
    pub manager_session_cwd: Option<String>,
    pub models: Table,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            projects: Vec::new(),
            brain_model: DEFAULT_MODEL.into(),
            brain_base_url: DEFAULT_BASE_URL.into(),
            brain_enabled: true,
            brain_reasoning_effort: "low".into(),
            manager_name: DEFAULT_MANAGER_NAME.into(),
            manager_title: DEFAULT_MANAGER_TITLE.into(),
            guard_words: None,
            rooms: Vec::new(),
            owner_names: Vec::new(),
            rooms_interval_s: 15.0,
            agents: Vec::new(),
            gates_enabled: true,
            github_token_env: None,
            github_agent_logins: Vec::new(),
            app_tirith: None,
            app_editor: None,
            launch_command: None,
            dispatch_enabled: true,
            dispatch_auto_launch: true,
            autopilot_enabled: true,
            autopilot_max_parallel: 4,
            review_auto_merge: true,
            disk_min_free_gb: 5.0,
            manager_session_cwd: None,
            models: Table::new(),
        }
    }
}

impl Config {
    pub fn room(&self, room_id: &str) -> Option<&RoomConfig> {
        self.rooms.iter().find(|r| r.id == room_id)
    }

    pub fn project(&self, project_id: &str) -> Option<&ProjectConfig> {
        self.projects.iter().find(|p| p.id == project_id)
    }

    /// Guard words for a project: per-project, then global, then the default.
    pub fn words_for(&self, proj: Option<&ProjectConfig>, default_words: &[String]) -> Vec<String> {
        if let Some(words) = proj.and_then(|p| p.guard_words.as_ref()).filter(|w| !w.is_empty()) {
            return words.clone();
        }
        if let Some(words) = self.guard_words.as_ref().filter(|w| !w.is_empty()) {
            return words.clone();
        }
        default_words.to_vec()
    }
}

fn table_at<'a>(raw: &'a Table, key: &str) -> Option<&'a Table> {
    raw.get(key).and_then(Value::as_table)
}

/// `table.get(key, default) is not False`.
fn not_false(table: Option<&Table>, key: &str) -> bool {
    !matches!(table.and_then(|t| t.get(key)), Some(Value::Boolean(false)))
}

fn nonblank_str(table: Option<&Table>, key: &str) -> Option<String> {
    let text = table?.get(key)?.as_str()?.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| items.iter().map(str_of).filter(|s| !crate::pytext::strip(s).is_empty()).collect())
        .unwrap_or_default()
}

/// Python `float(value)` for the TOML types it accepts.
fn as_float(value: &Value) -> Option<f64> {
    match value {
        Value::Integer(i) => Some(*i as f64),
        Value::Float(f) => Some(*f),
        Value::Boolean(b) => Some(f64::from(u8::from(*b))),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn project_from(table: &Table, fallback_id: Option<&str>) -> Option<ProjectConfig> {
    let id = get_str(table, "id").or_else(|| fallback_id.filter(|f| !f.is_empty()).map(String::from))?;
    let repo = get_str(table, "repo").or_else(|| get_str(table, "folder"));
    let words = table.get("guard_words").and_then(Value::as_array).filter(|a| !a.is_empty());
    Some(ProjectConfig {
        name: get_str(table, "name").unwrap_or_else(|| id.clone()),
        id,
        repo: repo.map(|r| expanduser(&r)),
        color: get_str(table, "color").unwrap_or_else(|| DEFAULT_COLOR.into()),
        guard_words: words.map(|w| w.iter().map(str_of).collect()),
        architecture: table_at(table, "architecture").cloned(),
        guard: not_false(Some(table), "guard"),
        cargo_target_dir: get_str(table, "cargo_target_dir").map(|d| expanduser(&d)),
    })
}

/// Read `<home>/config.toml`; a missing or broken file gives the defaults.
///
/// Projects may be `[[projects]]` tables or one `[projects.<id>]` table per project.
pub fn load_config(home: &Path) -> Config {
    let Ok(text) = fs::read_to_string(home.join("config.toml")) else { return Config::default() };
    match text.parse::<Table>() {
        Ok(raw) => config_from(&raw),
        Err(_) => Config::default(),
    }
}

fn config_from(raw: &Table) -> Config {
    let mut cfg = Config::default();
    match raw.get("projects") {
        Some(Value::Array(items)) => {
            cfg.projects.extend(items.iter().filter_map(Value::as_table).filter_map(|t| project_from(t, None)));
        }
        Some(Value::Table(map)) => {
            cfg.projects.extend(
                map.iter().filter_map(|(pid, item)| item.as_table().and_then(|t| project_from(t, Some(pid)))),
            );
        }
        _ => {}
    }
    let brain = table_at(raw, "brain");
    if let Some(model) = brain.and_then(|b| get_str(b, "model")) {
        cfg.brain_model = model;
    }
    if let Some(url) = brain.and_then(|b| get_str(b, "base_url")) {
        cfg.brain_base_url = url.trim_end_matches('/').to_string();
    }
    if let Some(enabled) = brain.and_then(|b| b.get("enabled")) {
        cfg.brain_enabled = truthy(enabled);
    }
    if let Some(effort) = brain.and_then(|b| b.get("reasoning_effort")) {
        cfg.brain_reasoning_effort = if truthy(effort) { str_of(effort) } else { String::new() };
    }
    // [manager]; the old [steward] table is the fallback
    let manager = ["manager", "steward"].iter().find_map(|k| table_at(raw, k));
    if let Some(name) = manager.and_then(|m| get_str(m, "name")) {
        cfg.manager_name = name;
    }
    if let Some(title) = manager.and_then(|m| get_str(m, "title")) {
        cfg.manager_title = title;
    }
    if let Some(cwd) = manager.and_then(|m| get_str(m, "session_cwd")) {
        cfg.manager_session_cwd = Some(expanduser(&cwd).to_string_lossy().into_owned());
    }
    let dispatch = table_at(raw, "dispatch");
    cfg.dispatch_enabled = not_false(dispatch, "enabled");
    cfg.dispatch_auto_launch = not_false(dispatch, "auto_launch");
    let autopilot = table_at(raw, "autopilot");
    cfg.autopilot_enabled = not_false(autopilot, "enabled");
    if let Some(n) = autopilot.and_then(|a| a.get("max_parallel")).and_then(Value::as_integer).filter(|n| *n >= 0) {
        cfg.autopilot_max_parallel = n;
    }
    cfg.review_auto_merge = not_false(table_at(raw, "review"), "auto_merge");
    if let Some(free) = table_at(raw, "disk").and_then(|d| d.get("min_free_gb")) {
        let number = match free {
            Value::Integer(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        };
        if let Some(n) = number.filter(|n| *n >= 0.0) {
            cfg.disk_min_free_gb = n;
        }
    }
    if let Some(agents) = table_at(raw, "agents") {
        cfg.agents = agents.iter().filter_map(|(a, t)| t.as_table().map(|t| (a.clone(), t.clone()))).collect();
    }
    cfg.gates_enabled = not_false(table_at(raw, "gates"), "enabled");
    let github = table_at(raw, "github");
    cfg.github_token_env = github.and_then(|g| get_str(g, "token_env"));
    cfg.github_agent_logins = string_list(github.and_then(|g| g.get("agent_logins")));
    let app = table_at(raw, "app");
    cfg.app_tirith = nonblank_str(app, "tirith");
    cfg.app_editor = nonblank_str(app, "editor");
    if let Some(command) = table_at(raw, "launch").and_then(|l| l.get("command")) {
        if matches!(command, Value::String(_) | Value::Array(_)) && truthy(command) {
            cfg.launch_command = Some(command.clone());
        }
    }
    if let Some(models) = table_at(raw, "models") {
        cfg.models = models.clone();
    }
    if let Some(rooms) = raw.get("rooms").and_then(Value::as_array) {
        for item in rooms.iter().filter_map(Value::as_table) {
            let Some(id) = get_str(item, "id") else { continue };
            cfg.rooms.push(RoomConfig {
                id,
                project: get_str(item, "project"),
                aliases: table_at(item, "aliases")
                    .map(|a| a.iter().map(|(k, v)| (k.clone(), str_of(v))).collect())
                    .unwrap_or_default(),
                title: get_str(item, "title"),
            });
        }
    }
    cfg.owner_names = string_list(table_at(raw, "owner").and_then(|o| o.get("names")));
    if let Some(interval) = table_at(raw, "messenger").and_then(|m| m.get("interval_s")).and_then(as_float) {
        cfg.rooms_interval_s = interval.max(10.0);
    }
    if let Some(words) = raw.get("guard_words").and_then(Value::as_array).filter(|w| !w.is_empty()) {
        cfg.guard_words = Some(words.iter().map(str_of).collect());
    }
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(text: &str) -> Config {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("config.toml"), text).unwrap();
        load_config(dir.path())
    }

    #[test]
    fn missing_and_broken_files_give_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_config(dir.path()), Config::default());
        assert_eq!(load("[[projects"), Config::default());
    }

    #[test]
    fn projects_in_both_forms() {
        let cfg = load("[[projects]]\nid = \"a\"\nrepo = \"/x/a\"\nguard = false\n[[projects]]\nname = \"no id\"\n");
        assert_eq!(cfg.projects.len(), 1);
        let a = &cfg.projects[0];
        assert_eq!((a.id.as_str(), a.name.as_str(), a.color.as_str()), ("a", "a", "#888888"));
        assert_eq!(a.repo, Some(PathBuf::from("/x/a")));
        assert!(!a.guard);

        let cfg = load("[projects.z]\nfolder = \"/z\"\n[projects.b]\nname = \"B\"\ncolor = \"#111111\"\n");
        let ids: Vec<_> = cfg.projects.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["z", "b"], "file order is kept");
        assert_eq!(cfg.projects[0].repo, Some(PathBuf::from("/z")));
        assert_eq!(cfg.projects[1].name, "B");
    }

    #[test]
    fn settings_and_wrong_types() {
        let cfg = load(
            "guard_words = [\"x\", \"y\"]\n[brain]\nmodel = \"m\"\nbase_url = \"http://h/v1/\"\nenabled = false\n\
             [steward]\nname = \"Old\"\n[autopilot]\nmax_parallel = -3\n[disk]\nmin_free_gb = \"lots\"\n\
             [messenger]\ninterval_s = 2\n[owner]\nnames = [\"me\", \" \"]\n\
             [[rooms]]\nid = \"r\"\n[rooms_ignored]\nx = 1\n[[rooms]]\ntitle = \"no id\"\n",
        );
        assert_eq!((cfg.brain_model.as_str(), cfg.brain_base_url.as_str(), cfg.brain_enabled), ("m", "http://h/v1", false));
        assert_eq!(cfg.manager_name, "Old");
        assert_eq!(cfg.autopilot_max_parallel, 4);
        assert_eq!(cfg.disk_min_free_gb, 5.0);
        assert_eq!(cfg.rooms_interval_s, 10.0);
        assert_eq!(cfg.owner_names, ["me"]);
        assert_eq!(cfg.rooms.len(), 1);
        assert_eq!(cfg.words_for(None, &["d".into()]), ["x", "y"]);
    }

    #[test]
    fn home_prefers_explicit_argument() {
        assert_eq!(tirith_home(Some(Path::new("/some/home"))), PathBuf::from("/some/home"));
    }
}
