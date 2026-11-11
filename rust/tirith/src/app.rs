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
    cfg_mtime: Mutex<Option<SystemTime>>,
}

pub type R = Result<axum::Json<Value>, (axum::http::StatusCode, String)>;

pub fn bad(e: String) -> (axum::http::StatusCode, String) {
    (axum::http::StatusCode::BAD_REQUEST, e)
}

/// Seconds since the lane's log last changed (`u64::MAX` without a log).
pub fn log_age(app: &Shared, id: &str) -> u64 {
    std::fs::metadata(app.home.join(format!("logs/lanes/{id}.log"))).and_then(|m| m.modified()).map(|t| t.elapsed().map(|d| d.as_secs()).unwrap_or(0)).unwrap_or(u64::MAX)
}
pub fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
pub fn now_s() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}
fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

impl App {
    pub fn open() -> Shared {
        let home = tirith_home();
        for dir in ["logs/lanes", "logs/runs", "briefs", "lanes"] {
            let _ = std::fs::create_dir_all(home.join(dir));
        }
        let db = Connection::open(home.join("tirith.sqlite3")).expect("open tirith.sqlite3");
        db.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=5000;
             CREATE TABLE IF NOT EXISTS lanes (id TEXT PRIMARY KEY, data TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS events (id INTEGER PRIMARY KEY, at TEXT, lane TEXT, project TEXT,
                 kind TEXT, tool TEXT, summary TEXT, session TEXT);
             CREATE TABLE IF NOT EXISTS chat (id INTEGER PRIMARY KEY, at TEXT, role TEXT, text TEXT);
             CREATE TABLE IF NOT EXISTS runs (id INTEGER PRIMARY KEY, lane TEXT, project TEXT, stage TEXT,
                 status TEXT, started_at TEXT, ended_at TEXT, log TEXT);
             CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value TEXT);
             CREATE TABLE IF NOT EXISTS pipelines (id TEXT PRIMARY KEY, project TEXT, kind TEXT, lanes TEXT, sha TEXT,
                 status TEXT, started_at TEXT, ended_at TEXT);
             CREATE TABLE IF NOT EXISTS owner_items (id TEXT PRIMARY KEY, at TEXT, kind TEXT, question TEXT,
                 status TEXT, answer TEXT);",
        )
        .expect("create tables");
        // Columns added later (each ALTER fails once its column exists). A chat row and an owner card belong to a project; '' is none.
        for (table, col) in [("runs", "pipeline TEXT"), ("runs", "sha TEXT"), ("chat", "project TEXT NOT NULL DEFAULT ''"), ("owner_items", "project TEXT NOT NULL DEFAULT ''")] {
            let _ = db.execute(&format!("ALTER TABLE {table} ADD COLUMN {col}"), []);
        }
        let lanes: BTreeMap<String, Lane> =
            db.prepare("SELECT data FROM lanes").expect("read lanes").query_map([], |r| r.get::<_, String>(0)).expect("read lanes").flatten().filter_map(|s| serde_json::from_str::<Lane>(&s).ok()).map(|l| (l.id.clone(), l)).collect();
        let cfg_mtime = mtime(&home.join("config.toml"));
        Arc::new(App {
            home,
            cfg: RwLock::new(Arc::new(Config::load())),
            db: Mutex::new(db),
            lanes: Mutex::new(lanes),
            tx: broadcast::channel(1024).0,
            plan_dirty: Mutex::new(BTreeSet::new()),
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

    /// Re-reads `config.toml` when it changed on disk (one stat) or when `force`d; true when it was re-read.
    pub fn reload_config(&self, force: bool) -> bool {
        let now = mtime(&self.home.join("config.toml"));
        let mut last = self.cfg_mtime.lock().unwrap();
        if *last == now && !force {
            return false;
        }
        *last = now;
        *self.cfg.write().unwrap() = Arc::new(Config::load());
        true
    }

    /// A project's `tirith.toml`, re-read only when its mtime changed.
    pub fn manifest(&self, repo: &Path) -> Manifest {
        let stamp = mtime(&repo.join("tirith.toml"));
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
        let _ = self.db.lock().unwrap().execute("INSERT INTO lanes (id, data) VALUES (?1, ?2) ON CONFLICT(id) DO UPDATE SET data = ?2", params![lane.id, data]);
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

    /// The working lane whose worktree holds `cwd`.
    pub fn lane_for_cwd(&self, cwd: &str) -> Option<Lane> {
        let cwd = Path::new(cwd);
        self.lanes.lock().unwrap().values().find(|l| matches!(l.status.as_str(), "running" | "waiting") && !l.workspace.is_empty() && cwd.starts_with(&l.workspace)).cloned()
    }

    /// Runs an INSERT and returns the id of the new row.
    pub fn insert(&self, sql: &str, values: &[&dyn rusqlite::ToSql]) -> i64 {
        let db = self.db.lock().unwrap();
        let _ = db.execute(sql, values);
        db.last_insert_rowid()
    }

    pub fn event(&self, lane: &str, project: &str, kind: &str, tool: &str, summary: &str, session: &str) {
        let at = now_iso();
        let id = self.insert("INSERT INTO events (at, lane, project, kind, tool, summary, session) VALUES (?1,?2,?3,?4,?5,?6,?7)", params![at, lane, project, kind, tool, summary, session]);
        self.emit("event", json!({"id": id, "at": at, "lane": lane, "project": project, "kind": kind, "tool": tool, "summary": summary}));
    }

    /// One row of a project's conversation with its manager (`project` "" is the conversation of no project).
    pub fn chat(&self, role: &str, text: &str, project: &str) {
        let project = if self.cfg().project(project).is_some() { project } else { "" }; // a project that is gone
        let at = now_iso();
        let id = self.insert("INSERT INTO chat (at, role, text, project) VALUES (?1,?2,?3,?4)", params![at, role, text, project]);
        self.emit("chat", json!({"id": id, "at": at, "role": role, "text": text, "project": project}));
    }

    pub fn kv(&self, key: &str) -> Option<String> {
        self.db.lock().unwrap().query_row("SELECT value FROM kv WHERE key = ?1", [key], |r| r.get(0)).ok()
    }

    pub fn set_kv(&self, key: &str, value: Option<&str>) {
        let db = self.db.lock().unwrap();
        let _ = match value {
            Some(v) => db.execute("INSERT OR REPLACE INTO kv (key, value) VALUES (?1, ?2)", params![key, v]),
            None => db.execute("DELETE FROM kv WHERE key = ?1", [key]),
        };
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
            let mut obj = serde_json::Map::new();
            for (i, c) in cols.iter().enumerate() {
                let v = r.get_ref(i)?;
                obj.insert(c.to_string(), v.as_i64().map(|n| json!(n)).or_else(|_| v.as_str().map(|s| json!(s))).unwrap_or(Value::Null));
            }
            Ok(Value::Object(obj))
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
    for dir in std::iter::once(cargo).chain(tool_dirs.iter().map(PathBuf::from)) {
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
    if out.status.success() {
        Ok(text(&out.stdout))
    } else {
        Err(format!("git {}: {} {}", args.join(" "), text(&out.stderr), text(&out.stdout)).trim().to_string())
    }
}

/// A worktree at `base` (a new branch when `branch` is given), without the manifest's `lane_exclude` paths.
pub async fn add_worktree(repo: &Path, dir: &Path, branch: Option<&str>, exclude: &[String]) -> Result<(), String> {
    let _ = dir.parent().map(std::fs::create_dir_all);
    let d = dir.to_string_lossy().to_string();
    let mut args = vec!["worktree", "add"];
    if !exclude.is_empty() {
        args.push("--no-checkout");
    }
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
    for rel in git(repo, &args).await.unwrap_or_default().split('\0').filter(|r| !r.is_empty()) {
        let to = dir.join(rel);
        let _ = to.parent().map(std::fs::create_dir_all).map(|_| std::fs::copy(repo.join(rel), &to));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lane_log_redacts_key_prefixes() {
        let log = "GOOGLE_API_KEY=AIzaSyABCDEFGHIJKLMNOP\n{\"GITHUB_TOKEN\":\"ghp_abcdefghijklmnop12\"}\nMY_KEY: sk-abcdef123456789\nused AIzaSyZZZZZZZZZZZZZZ here\nplain line";
        let out = redact_secrets(log);
        ["AIza", "ghp_", "sk-", "abcdefghijklmnop12"].into_iter().for_each(|leak| assert!(!out.contains(leak), "leaked {leak}: {out}"));
        assert!(out.contains("GOOGLE_API_KEY=[redacted]") && out.contains("plain line"));
    }
    #[test]
    fn child_path_includes_cargo_bin() {
        let home = std::env::temp_dir().join(format!("tirith-cp-{}", std::process::id()));
        let cargo = home.join(".cargo/bin");
        std::fs::create_dir_all(&cargo).unwrap();
        let p = path_with_cargo(Some(Path::new("/x")), "/usr/bin", &home, &[]);
        assert_eq!(p, format!("/x:/usr/bin:{}", cargo.display()));
        assert_eq!(path_with_cargo(None, &format!("/a:{}", cargo.display()), &home, &[]), format!("/a:{}", cargo.display()));
        let _ = std::fs::remove_dir_all(&home);
        assert_eq!(path_with_cargo(None, "/usr/bin", &home, &[]), "/usr/bin");
    }
    #[test]
    fn child_path_includes_homebrew_bin() {
        let base = std::env::temp_dir().join(format!("tirith-hb-{}", std::process::id()));
        let brew = base.join("brew/bin");
        let dirs = [brew.to_str().unwrap(), "/nonexistent/bin"];
        std::fs::create_dir_all(&brew).unwrap();
        let p = path_with_cargo(None, "/usr/bin", &base, &dirs);
        assert_eq!((p.clone(), path_with_cargo(None, &p, &base, &dirs)), (format!("/usr/bin:{}", brew.display()), p));
    }
    /// A repository with one commit holding `files` (each a folder-relative path and its text).
    async fn committed(repo: &Path, files: &[(&str, &str)]) {
        for (file, text) in files {
            let path = repo.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).and_then(|_| std::fs::write(path, text)).unwrap();
        }
        for a in ["init -q", "config user.email t@t", "config user.name t", "add .", "commit -qm i"] {
            git(repo, &a.split(' ').collect::<Vec<_>>()).await.unwrap();
        }
    }
    #[tokio::test]
    async fn worktree_local_properties() {
        let base = std::env::temp_dir().join(format!("tirith-lp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let repo = base.join("repo");
        committed(&repo, &[(".gitignore", "local.properties\n"), ("README", "x")]).await;
        std::fs::create_dir_all(repo.join("android")).unwrap();
        std::fs::write(repo.join("android/local.properties"), "sdk.dir=/sdk\n").unwrap();
        let wt = base.join("wt");
        add_worktree(&repo, &wt, None, &[]).await.unwrap();
        assert_eq!(std::fs::read_to_string(wt.join("android/local.properties")).unwrap(), "sdk.dir=/sdk\n");
        let _ = std::fs::remove_dir_all(&base);
    }
    #[tokio::test]
    async fn sparse_zoo_lane() {
        let repo = std::env::temp_dir().join(format!("tirith-sp-{}", std::process::id()));
        let ex: Vec<String> = ["output/research", "tools/content", "ZooBundles"].map(String::from).into();
        let files: Vec<(String, &str)> = ex.iter().map(String::as_str).chain(["cms"]).map(|d| (format!("{d}/f"), "x")).collect();
        committed(&repo, &files.iter().map(|(f, t)| (f.as_str(), *t)).collect::<Vec<_>>()).await;
        let wt = repo.join("../wt-sp");
        add_worktree(&repo, &wt, Some("lane/z"), &ex).await.unwrap();
        assert!(wt.join("cms/f").exists() && ex.iter().all(|p| !wt.join(p).exists()));
        let _ = std::fs::remove_dir_all(&wt);
        let _ = std::fs::remove_dir_all(&repo);
    }
}

/// Signals a process group; pid 0 would be the daemon's own group, so it is never signalled.
pub fn kill_group(pid: u32, signal: i32) {
    if pid != 0 {
        unsafe { libc::kill(-(pid as i32), signal) };
    }
}

pub fn alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

/// Hides whole secret values in log text: the value of any `*KEY*`/`*TOKEN*`/`*SECRET*`/`*PASSWORD*` variable
/// (`NAME=value`, `NAME: value`, `"NAME":"value"`) and bare well-known key shapes. Nothing of the value is kept,
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
