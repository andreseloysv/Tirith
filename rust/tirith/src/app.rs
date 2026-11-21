//! The daemon's shared state: config, SQLite, the lanes in memory and the change feed the page listens to.

use crate::config::{tirith_home, Config, Manifest};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, Notify};

pub type Shared = Arc<App>;

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Lane {
    pub id: String,
    pub project: String,
    pub step: String,
    pub item: String,
    pub agent: String,
    pub model: Option<String>,
    /// queued, running, waiting (asked the manager), merge-queued, merging, waiting-disk (outside the queue), merged, failed, stopped
    pub status: String,
    /// Its worktree slot (`<tirith home>/lanes/<project>/slot-N`), kept until the lane is merged, failed or stopped.
    pub workspace: String,
    pub branch: String,
    pub pid: Option<u32>,
    pub attempts: u32,
    /// The agent's last message (Stop hook) or Tirith's note.
    pub note: String,
    /// The agent session id seen in its hooks (a resume continues it).
    pub session: String,
    pub question: String,
    /// The message the next run starts with: resumes `session` when there is one, else ends the brief.
    pub resume: String,
    /// Agent aliases already used (out-of-credits fallback).
    pub tried: Vec<String>,
    /// Merged alone (its train was red).
    pub solo: bool,
    /// Its factory stamp: the factory version and the hash of the lane role's setting it was created with.
    pub factory: String,
    pub role_hash: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub merged_at: Option<String>,
    pub last_activity: u64,
}

impl Lane {
    pub fn active(&self) -> bool {
        matches!(self.status.as_str(), "queued" | "running" | "waiting" | "merge-queued" | "merging")
    }
}

pub struct App {
    pub home: PathBuf,
    pub cfg: RwLock<Arc<Config>>,
    pub db: Mutex<Connection>,
    pub lanes: Mutex<BTreeMap<String, Lane>>,
    pub tx: broadcast::Sender<String>,
    pub plan_dirty: Mutex<BTreeSet<String>>,
    pub plan_lock: tokio::sync::Mutex<()>,
    pub sched_wake: Notify,
    pub merge_wake: Notify,
    pub children: Mutex<HashSet<String>>,
    manifests: Mutex<HashMap<PathBuf, (Option<SystemTime>, Manifest)>>,
    /// The last-seen changes of (config.toml, the factory folder).
    cfg_mtime: Mutex<(Option<SystemTime>, Option<SystemTime>)>,
}

pub type R = Result<axum::Json<Value>, (axum::http::StatusCode, String)>;

pub fn bad(e: String) -> (axum::http::StatusCode, String) {
    (axum::http::StatusCode::BAD_REQUEST, e)
}

pub fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
pub fn now_s() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl App {
    pub fn open() -> Shared {
        let home = tirith_home();
        ["logs/lanes", "logs/runs", "briefs", "lanes"].into_iter().for_each(|dir| drop(std::fs::create_dir_all(home.join(dir))));
        let db = Connection::open(home.join("tirith.sqlite3")).expect("open tirith.sqlite3");
        db.execute_batch(include_str!("../assets/schema.sql")).expect("create tables");
        // Columns added later (each ALTER fails once its column exists). A chat row and an owner card belong to a project; '' is none. A card may carry a waiting lane's question. Every stamped table carries its factory version and role hash.
        let owner = [("owner_items", "project TEXT NOT NULL DEFAULT ''"), ("owner_items", "lane TEXT NOT NULL DEFAULT ''")];
        [("runs", "pipeline TEXT"), ("runs", "sha TEXT"), ("chat", "project TEXT NOT NULL DEFAULT ''")].into_iter().chain(owner).for_each(|(table, col)| drop(db.execute(&format!("ALTER TABLE {table} ADD COLUMN {col}"), [])));
        ["lanes", "events", "chat", "owner_items", "ceo_reviews"].into_iter().for_each(|t| ["factory", "role_hash"].into_iter().for_each(|c| drop(db.execute(&format!("ALTER TABLE {t} ADD COLUMN {c} TEXT NOT NULL DEFAULT ''"), []))));
        let lanes: BTreeMap<String, Lane> =
            db.prepare("SELECT data FROM lanes").expect("read lanes").query_map([], |r| r.get::<_, String>(0)).expect("read lanes").flatten().filter_map(|s| serde_json::from_str::<Lane>(&s).ok()).map(|l| (l.id.clone(), l)).collect();
        let cfg_mtime = std::fs::metadata(home.join("config.toml")).and_then(|m| m.modified()).ok();
        let cfg = Arc::new(Config::load());
        let cfg_mtime = (cfg_mtime, crate::factory::stamp(&cfg.raw));
        Arc::new(App {
            home,
            cfg: RwLock::new(cfg.clone()),
            db: Mutex::new(db),
            lanes: Mutex::new(lanes),
            tx: broadcast::channel(1024).0,
            plan_dirty: Mutex::new(cfg.projects.iter().map(|p| p.id.clone()).collect()), // every project starts dirty: the first flush commits a plan a stopped daemon left uncommitted (flush skips a clean one)
            plan_lock: tokio::sync::Mutex::new(()),
            sched_wake: Notify::new(),
            merge_wake: Notify::new(),
            children: Mutex::new(HashSet::new()),
            manifests: Mutex::new(HashMap::new()),
            cfg_mtime: Mutex::new(cfg_mtime),
        })
    }

    pub fn cfg(&self) -> Arc<Config> {
        self.cfg.read().unwrap().clone()
    }

    /// Re-reads `config.toml` and the factory when either changed on disk or main moved (a few stats, see `factory::stamp`) or when `force`d; true when re-read. A bad factory version keeps the last good one (see `factory::load`).
    pub fn reload_config(&self, force: bool) -> bool {
        let now = (std::fs::metadata(self.home.join("config.toml")).and_then(|m| m.modified()).ok(), crate::factory::stamp(&self.cfg().raw));
        let mut last = self.cfg_mtime.lock().unwrap();
        if *last == now && !force {
            return false;
        }
        *last = now;
        drop(last);
        *self.cfg.write().unwrap() = Arc::new(Config::load());
        true
    }

    /// A project's `tirith.toml`, re-read only when its mtime changed.
    pub fn manifest(&self, repo: &Path) -> Manifest {
        let stamp = std::fs::metadata(repo.join("tirith.toml")).and_then(|m| m.modified()).ok();
        let mut cache = self.manifests.lock().unwrap();
        if let Some((_, man)) = cache.get(repo).filter(|(m, _)| *m == stamp) {
            return man.clone();
        }
        let man = Manifest::load(repo);
        cache.insert(repo.to_path_buf(), (stamp, man.clone()));
        man
    }

    pub fn emit(&self, t: &str, d: Value) {
        let _ = self.tx.send(json!({ "t": t, "d": d }).to_string());
    }

    pub fn lane(&self, id: &str) -> Option<Lane> {
        self.lanes.lock().unwrap().get(id).cloned()
    }

    pub fn save_lane(&self, lane: &Lane) {
        let data = serde_json::to_string(lane).unwrap_or_default();
        self.insert("INSERT INTO lanes (id, data, factory, role_hash) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(id) DO UPDATE SET data = ?2, factory = ?3, role_hash = ?4", params![lane.id, data, lane.factory, lane.role_hash]);
        self.lanes.lock().unwrap().insert(lane.id.clone(), lane.clone());
        self.emit("lane", json!(lane));
    }

    /// Changes one lane in place (memory, SQLite, page) and returns the new value.
    pub fn update_lane(&self, id: &str, f: impl FnOnce(&mut Lane)) -> Option<Lane> {
        let mut lane = self.lane(id)?;
        f(&mut lane);
        self.save_lane(&lane);
        Some(lane)
    }

    pub fn insert(&self, sql: &str, values: &[&dyn rusqlite::ToSql]) -> i64 {
        let db = self.db.lock().unwrap();
        let _ = db.execute(sql, values);
        db.last_insert_rowid()
    }

    /// The conversation of `project` (a project or a bot), or the one of no project when there is no such project (any more).
    pub fn conv<'a>(&self, project: &'a str) -> &'a str {
        if self.cfg().project(project).is_some() || crate::bots::bot(self, project).is_some() {
            project
        } else {
            ""
        }
    }

    /// The factory role of a conversation: a bot's own key, else the `session = "chat"` role of its scope (a project's, or the global one of no project).
    pub fn chat_role(&self, project: &str) -> String {
        if crate::bots::bot(self, project).is_some() {
            return project.to_string();
        }
        let (scope, fallback) = if self.cfg().project(project).is_some() { ("project", "manager") } else { ("global", "lead") };
        self.cfg().factory.find_role("chat", Some(scope)).unwrap_or_else(|| fallback.into())
    }

    /// The stamp a row made now by `role` carries: (the factory version, a hash of the role's setting with the page's model override for it folded in). A name that is no factory role (the owner, a room, Tirith itself) keeps an empty hash.
    pub fn stamp_for(&self, role: &str) -> (String, String) {
        let cfg = self.cfg();
        (cfg.factory.version.clone(), cfg.factory.role_hash(role, &self.kv(&format!("models:{role}")).unwrap_or_default()))
    }

    /// One event row, shown on the page; its id, so an eval's score can keep it.
    pub fn event(&self, lane: &str, project: &str, kind: &str, tool: &str, summary: &str, session: &str) -> i64 {
        let at = now_iso();
        let (factory, role_hash) = (self.cfg().factory.version.clone(), self.lane(lane).map(|l| l.role_hash).unwrap_or_default()); // a lane's event carries its lane's stamp
        let id = self.insert("INSERT INTO events (at, lane, project, kind, tool, summary, session, factory, role_hash) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)", params![at, lane, project, kind, tool, summary, session, factory, role_hash]);
        self.emit("event", json!({"id": id, "at": at, "lane": lane, "project": project, "kind": kind, "tool": tool, "summary": summary, "factory": factory, "role_hash": role_hash}));
        id
    }

    /// One row of a project's conversation with its manager (`project` "" is the conversation of no project). A row a role wrote while working (its turn's text and tool calls, a turn role's answer under its name, a bot's routine) carries that role's stamp; every row carries the factory version.
    pub fn chat(&self, role: &str, text: &str, project: &str) {
        let project = self.conv(project); // a project or bot that is gone talks in the conversation of no project
        let at = now_iso();
        let by = if matches!(role, "manager" | "tool" | "routine") { self.chat_role(project) } else { role.to_string() };
        let (factory, role_hash) = self.stamp_for(&by);
        let id = self.insert("INSERT INTO chat (at, role, text, project, factory, role_hash) VALUES (?1,?2,?3,?4,?5,?6)", params![at, role, text, project, factory, role_hash]);
        self.emit("chat", json!({"id": id, "at": at, "role": role, "text": text, "project": project, "factory": factory, "role_hash": role_hash}));
    }

    pub fn kv(&self, key: &str) -> Option<String> {
        self.db.lock().unwrap().query_row("SELECT value FROM kv WHERE key = ?1", [key], |r| r.get(0)).ok()
    }

    pub fn set_kv(&self, key: &str, value: Option<&str>) {
        let db = self.db.lock().unwrap();
        let _ = value.map_or_else(|| db.execute("DELETE FROM kv WHERE key = ?1", [key]), |v| db.execute("INSERT OR REPLACE INTO kv (key, value) VALUES (?1, ?2)", params![key, v]));
    }

    /// The rows of a query, each mapped by `f`; a query that fails gives none.
    pub fn query<T>(&self, sql: &str, args: impl rusqlite::Params, f: impl FnMut(&rusqlite::Row) -> rusqlite::Result<T>) -> Vec<T> {
        let db = self.db.lock().unwrap();
        let Ok(mut q) = db.prepare(sql) else { return vec![] };
        q.query_map(args, f).map(|rows| rows.flatten().collect()).unwrap_or_default()
    }

    /// The rows of a query as JSON objects named by `cols` (numbers and text; anything else is null).
    pub fn objects(&self, sql: &str, args: impl rusqlite::Params, cols: &[&str]) -> Vec<Value> {
        self.query(sql, args, |r| {
            cols.iter()
                .enumerate()
                .map(|(i, c)| r.get_ref(i).map(|v| (c.to_string(), v.as_i64().map(|n| json!(n)).or_else(|_| v.as_str().map(|s| json!(s))).unwrap_or(Value::Null))))
                .collect::<rusqlite::Result<serde_json::Map<String, Value>>>()
                .map(Value::Object)
        })
    }

    /// The last `limit` rows of a table (with `project`, only that project's) as JSON objects, oldest first.
    pub fn recent(&self, table: &str, cols: &[&str], project: Option<&str>, limit: u32) -> Vec<Value> {
        let only = if project.is_some() { "WHERE project = ?1" } else { "" };
        let mut out = self.objects(&format!("SELECT {} FROM {table} {only} ORDER BY id DESC LIMIT {limit}", cols.join(", ")), rusqlite::params_from_iter(project), cols);
        out.reverse();
        out
    }

    /// Keeps the event table bounded: the newest 50 000 rows.
    pub fn trim(&self) {
        let _ = self.db.lock().unwrap().execute("DELETE FROM events WHERE id < (SELECT COALESCE(MAX(id), 0) - 50000 FROM events)", []);
    }

    /// PATH for agents and checks: this binary's folder first, so `tirith` is this daemon's client, and `~/.cargo/bin`, `/opt/homebrew/bin`, `/usr/local/bin` last when they exist.
    pub fn child_path(&self) -> String {
        let exe_dir = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf));
        path_with_cargo(exe_dir.as_deref(), &std::env::var("PATH").unwrap_or_default(), &crate::config::home_dir(), &["/opt/homebrew/bin", "/usr/local/bin"])
    }
}

fn path_with_cargo(exe_dir: Option<&Path>, rest: &str, home: &Path, tool_dirs: &[&str]) -> String {
    let cargo = home.join(".cargo/bin");
    let mut parts: Vec<String> = exe_dir.iter().map(|d| d.display().to_string()).chain(rest.split(':').filter(|p| !p.is_empty()).map(String::from)).collect();
    for dir in [cargo, home.join(".local/bin")].into_iter().chain(tool_dirs.iter().map(PathBuf::from)) {
        if dir.is_dir() && !parts.iter().any(|p| Path::new(p) == dir) {
            parts.push(dir.display().to_string());
        }
    }
    parts.join(":")
}

/// `git -C dir args…`: stdout on success, the error text otherwise.
pub async fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = tokio::process::Command::new("git").arg("-C").arg(dir).args(args).stdin(std::process::Stdio::null()).output().await.map_err(|e| e.to_string())?;
    let text = |b: &[u8]| String::from_utf8_lossy(b).trim().to_string();
    out.status.success().then(|| text(&out.stdout)).ok_or_else(|| format!("git {}: {} {}", args.join(" "), text(&out.stderr), text(&out.stdout)).trim().to_string())
}

/// A worktree at `base` (a new branch when `branch` is given), without the manifest's `lane_exclude` paths.
pub async fn add_worktree(repo: &Path, dir: &Path, branch: Option<&str>, exclude: &[String]) -> Result<(), String> {
    let _ = dir.parent().map(std::fs::create_dir_all);
    let d = dir.to_string_lossy().to_string();
    let mut args = vec!["worktree", "add"];
    args.extend((!exclude.is_empty()).then_some("--no-checkout"));
    match branch {
        Some(b) => args.extend(["-b", b, &d, "HEAD"]),
        None => args.extend(["--detach", &d, "HEAD"]),
    }
    git(repo, &args).await?;
    if !exclude.is_empty() {
        let mut sparse: Vec<String> = vec!["sparse-checkout".into(), "set".into(), "--no-cone".into(), "/*".into()];
        sparse.extend(exclude.iter().map(|p| format!("!/{}/", p.trim_matches('/'))));
        git(dir, &sparse.iter().map(String::as_str).collect::<Vec<_>>()).await?;
        git(dir, &["checkout", "-q"]).await?;
    }
    // Copy the git-ignored `local.properties` files (Android `sdk.dir`) from the main checkout into the new worktree.
    let args = ["ls-files", "--others", "--ignored", "--exclude-standard", "-z", "--", ":(glob)**/local.properties"];
    git(repo, &args).await.unwrap_or_default().split('\0').filter(|r| !r.is_empty()).for_each(|rel| drop(dir.join(rel).parent().map(|p| (std::fs::create_dir_all(p), std::fs::copy(repo.join(rel), dir.join(rel))))));
    Ok(())
}

#[cfg(test)]
#[path = "../checks/app.rs"]
mod tests;

/// Signals a process group; pid 0 would be the daemon's own group, so it is never signalled.
pub fn kill_group(pid: u32, signal: i32) {
    let _ = (pid != 0).then(|| unsafe { libc::kill(-(pid as i32), signal) });
}

pub fn alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

/// Hides whole secret values in log text: the value of any `*KEY*`/`*TOKEN*`/`*SECRET*`/`*PASSWORD*` variable (`NAME=value`, `NAME: value`, `"NAME":"value"`) and bare well-known key shapes. Nothing of the value is kept,
/// not even its first characters.
pub fn redact_secrets(text: &str) -> String {
    use regex::Regex;
    use std::sync::LazyLock;
    static NAMED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)(\b[A-Za-z0-9_]*(?:KEY|TOKEN|SECRET|PASSWORD)[A-Za-z0-9_]*\\?["']?\s*[=:]\s*\\?["']?)([^\s"'\\,;}&]+)"#).unwrap());
    static BARE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bAIza[0-9A-Za-z_\-]{10,}|\bsk-[0-9A-Za-z_\-]{10,}|\bgh[pousr]_[0-9A-Za-z]{10,}|\bxox[abpr]-[0-9A-Za-z\-]{10,}|\bAKIA[0-9A-Z]{12,}").unwrap());
    let step = NAMED.replace_all(text, "${1}[redacted]");
    BARE.replace_all(&step, "[redacted]").into_owned()
}

/// The last `lines` lines of a file.
pub fn tail(path: &Path, lines: usize) -> String {
    let text = std::fs::read(path).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}
