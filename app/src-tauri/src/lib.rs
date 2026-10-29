//! Tirith desktop app: the web UI in `web/` plus a Rust side that replaces the state polling with
//! Tauri events, keeps a collector running, launches steps and hosts the (flagged) chat.
//! No servers and no open ports: everything between the page and Rust is Tauri IPC.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder};

mod liveweb;

const STATE_EVENT: &str = "tirith://state";
const ALIVE_EVENT: &str = "tirith://alive";
const CHAT_EVENT: &str = "tirith://chat";

/// A chat session: the headless agent process and the lane that stands for it.
struct ChatSession {
    child: Child,
    stdin: ChildStdin,
}

#[derive(Default)]
struct App {
    /// The collector this app started (None when one was already running).
    collector: Mutex<Option<Child>>,
    chats: Mutex<HashMap<String, ChatSession>>,
    /// Action ids with a run in progress (one run at a time per action).
    running: Mutex<HashSet<String>>,
}

// -- paths and config ---------------------------------------------------------------------------

fn expand(p: &str) -> PathBuf {
    match p.strip_prefix("~/") {
        Some(rest) => user_home().join(rest),
        None => PathBuf::from(p),
    }
}

/// The repo folder of a project in config.toml (the default folder an owner action runs in).
fn project_repo(cfg: &toml::Table, project: &str) -> Option<PathBuf> {
    cfg.get("projects")?
        .as_array()?
        .iter()
        .find(|p| p.get("id").and_then(|i| i.as_str()) == Some(project))?
        .get("repo")?
        .as_str()
        .map(expand)
}

fn user_home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default()
}

/// `$TIRITH_HOME`, else `~/.tirith`.
fn tirith_home() -> PathBuf {
    std::env::var_os("TIRITH_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| user_home().join(".tirith"))
}

fn config() -> toml::Table {
    std::fs::read_to_string(tirith_home().join("config.toml"))
        .ok()
        .and_then(|t| t.parse::<toml::Table>().ok())
        .unwrap_or_default()
}

/// `[app]` setting as a string.
fn app_setting(cfg: &toml::Table, key: &str) -> Option<String> {
    cfg.get("app")?.get(key)?.as_str().map(str::to_string)
}

/// The collector folder: `$TIRITH_COLLECTOR_DIR`, `[app] collector_dir`, else the repo's `collector/`.
fn collector_dir(cfg: &toml::Table) -> PathBuf {
    if let Some(d) = std::env::var_os("TIRITH_COLLECTOR_DIR") {
        return PathBuf::from(d);
    }
    if let Some(d) = app_setting(cfg, "collector_dir") {
        return expand(&d);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../collector")
}

/// The folder the window serves the page from at runtime: `[app] web_dir`, else (debug builds only)
/// this repo's `web/`. None means the page embedded in the build (release builds without `web_dir`).
fn web_dir(cfg: &toml::Table) -> Option<PathBuf> {
    let dir = match app_setting(cfg, "web_dir") {
        Some(d) => expand(&d),
        None if cfg!(debug_assertions) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web"),
        None => return None,
    };
    dir.join("index.html").is_file().then_some(dir)
}

/// Tell the page when the files under `root` change (checked every 2 s).
fn spawn_web_watch(app: AppHandle, root: PathBuf) {
    std::thread::spawn(move || {
        let mut seen = liveweb::signature(&root);
        loop {
            std::thread::sleep(Duration::from_secs(2));
            let now = liveweb::signature(&root);
            if now != seen {
                seen = now;
                let _ = app.emit(liveweb::CHANGED_EVENT, ());
            }
        }
    });
}

/// Quotes one word for `/bin/sh`.
fn sh_quote(word: &str) -> String {
    if !word.is_empty() && word.chars().all(|c| c.is_ascii_alphanumeric() || "/._-+=:@%".contains(c)) {
        word.to_string()
    } else {
        format!("'{}'", word.replace('\'', "'\\''"))
    }
}

/// The argv that runs the collector CLI from anywhere: `[app] tirith`, else the repo's `bin/tirith`, else
/// `uv run --project <collector> tirith`.
fn tirith_cmd_argv(cfg: &toml::Table) -> Vec<String> {
    if let Some(t) = app_setting(cfg, "tirith").filter(|t| !t.trim().is_empty()) {
        return vec![expand(&t).to_string_lossy().into_owned()];
    }
    let script = collector_dir(cfg).join("../bin/tirith");
    if script.is_file() {
        let script = script.canonicalize().unwrap_or(script);
        return vec![script.to_string_lossy().into_owned()];
    }
    let uv = app_setting(cfg, "uv").unwrap_or_else(|| "uv".into());
    let dir = collector_dir(cfg).to_string_lossy().into_owned();
    vec![uv, "run".into(), "--project".into(), dir, "tirith".into()]
}

/// The same, as one shell-ready string (the resolved `tirith`).
fn tirith_cmd() -> String {
    tirith_cmd_argv(&config()).iter().map(|w| sh_quote(w)).collect::<Vec<_>>().join(" ")
}

/// `command` with each bare leading `tirith` word (line start, or after `&&` `||` `;` `|` `(`) replaced by
/// `cmd`. Mirrors `tirithcmd.rewrite` of the collector.
fn rewrite_tirith(command: &str, cmd: &str) -> String {
    let mut out = String::with_capacity(command.len());
    let mut start = true; // at the start of a command
    let mut rest = command;
    while let Some(ch) = rest.chars().next() {
        if start && (ch == ' ' || ch == '\t') {
            out.push(ch);
            rest = &rest[1..];
            continue;
        }
        if start && rest.starts_with("tirith") && rest[6..].chars().next().map_or(true, |a| a == ' ' || a == '\t' || a == '\n') {
            out.push_str(cmd);
            rest = &rest[6..];
            start = false;
            continue;
        }
        start = matches!(ch, '\n' | ';' | '|' | '(');
        if rest.starts_with("&&") {
            out.push_str("&&");
            rest = &rest[2..];
            start = true;
            continue;
        }
        out.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    out
}

/// PATH for children: the folder of `[app] uv`, then the usual tool folders, then the inherited PATH
/// (apps opened from Finder only get `/usr/bin:/bin:/usr/sbin:/sbin`). Duplicates are dropped.
fn child_path(uv: Option<&str>, home: &std::path::Path, inherited: Option<&std::ffi::OsStr>) -> std::ffi::OsString {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(u) = uv {
        if let Some(dir) = expand(u).parent().filter(|d| !d.as_os_str().is_empty()) {
            dirs.push(dir.to_path_buf());
        }
    }
    dirs.push("/opt/homebrew/bin".into());
    dirs.push("/usr/local/bin".into());
    dirs.push(home.join(".local/bin"));
    dirs.push(home.join(".cargo/bin"));
    if let Some(p) = inherited {
        dirs.extend(std::env::split_paths(p));
    }
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|d| seen.insert(d.clone()));
    std::env::join_paths(dirs).unwrap_or_default()
}

/// Gives `c` the PATH from `child_path`.
fn with_child_path(c: &mut Command) {
    let uv = app_setting(&config(), "uv");
    c.env("PATH", child_path(uv.as_deref(), &user_home(), std::env::var_os("PATH").as_deref()));
}

/// `tirith <args>` by the resolved command, ready to spawn.
fn tirith_command(args: &[&str]) -> Command {
    let argv = tirith_cmd_argv(&config());
    let mut c = Command::new(&argv[0]);
    c.args(&argv[1..]).args(args);
    c.env("TIRITH_HOME", tirith_home());
    with_child_path(&mut c);
    c
}

/// Appends a line to `~/.tirith/logs/<name>`; best effort.
fn log_line(name: &str, line: &str) {
    use std::io::Write;
    let dir = user_home().join(".tirith/logs");
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join(name)) {
        let _ = writeln!(f, "{line}");
    }
}

/// `~/.tirith/logs/collector.log`, opened for append, as stdout and stderr of the collector.
fn collector_log() -> Option<(Stdio, Stdio)> {
    let dir = user_home().join(".tirith/logs");
    std::fs::create_dir_all(&dir).ok()?;
    let f = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("collector.log")).ok()?;
    let g = f.try_clone().ok()?;
    Some((Stdio::from(f), Stdio::from(g)))
}

// -- state -> page ------------------------------------------------------------------------------

fn read_state() -> Option<Value> {
    let text = std::fs::read_to_string(tirith_home().join("state.json")).ok()?;
    serde_json::from_str(&text).ok()
}

/// One read for the page's first paint; later updates arrive as events.
#[tauri::command]
fn current_state() -> Option<Value> {
    read_state()
}

/// File name of an agent's detail script: the id with every character outside `[A-Za-z0-9_.-]` (and a
/// leading dot) replaced by `_`, plus `.js`. Mirrors `agent_file_name` in the collector.
fn agent_file_name(id: &str) -> String {
    let mut safe: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' { c } else { '_' })
        .collect();
    if safe.is_empty() {
        safe.push('_');
    } else if safe.starts_with('.') {
        safe.replace_range(..1, "_");
    }
    format!("{safe}.js")
}

/// The detail of `id` from `<home>/agents/<id>.js` (`window.TIRITH_AGENT = {...};`) as JSON.
fn agent_detail_in(home: &std::path::Path, id: &str) -> Result<Value, String> {
    let path = home.join("agents").join(agent_file_name(id));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("no detail for {id}: {e}"))?;
    let body = text
        .trim()
        .strip_prefix("window.TIRITH_AGENT")
        .map(|t| t.trim_start().trim_start_matches('=').trim().trim_end_matches(';'))
        .ok_or_else(|| format!("{} is not an agent detail file", path.display()))?;
    serde_json::from_str(body).map_err(|e| format!("bad detail file for {id}: {e}"))
}

/// The full log, events, files, tests and report of one agent, for the drawer.
#[tauri::command]
fn agent_detail(id: String) -> Result<Value, String> {
    agent_detail_in(&tirith_home(), &id)
}

/// What the pump sends for a freshly read state: the whole state only when `generated_at` moved AND the
/// content (everything but `generated_at`) changed; a small "alive" tick when only the clock moved.
#[derive(Debug, PartialEq)]
enum Push {
    State,
    Alive(String),
    Nothing,
}

#[derive(Default)]
struct Pump {
    generated_at: Option<String>,
    hash: Option<u64>,
}

impl Pump {
    fn decide(&mut self, state: &Value) -> Push {
        use std::hash::{Hash, Hasher};
        let generated = state["generated_at"].as_str().unwrap_or("").to_string();
        if self.generated_at.as_deref() == Some(generated.as_str()) {
            return Push::Nothing;
        }
        let mut rest = state.clone();
        if let Some(o) = rest.as_object_mut() {
            o.remove("generated_at");
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        rest.to_string().hash(&mut hasher);
        let hash = hasher.finish();
        self.generated_at = Some(generated.clone());
        if self.hash == Some(hash) {
            return Push::Alive(generated);
        }
        self.hash = Some(hash);
        Push::State
    }
}

/// Reads the state every 2 s and pushes it to the page only when it changed (see `Pump`).
fn spawn_state_pump(app: AppHandle) {
    std::thread::spawn(move || {
        let mut pump = Pump::default();
        loop {
            if let Some(s) = read_state() {
                match pump.decide(&s) {
                    Push::State => {
                        let _ = app.emit(STATE_EVENT, s);
                    }
                    Push::Alive(at) => {
                        let _ = app.emit(ALIVE_EVENT, at);
                    }
                    Push::Nothing => {}
                }
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

#[cfg(test)]
mod state_tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tirith-detail-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("agents")).unwrap();
        d
    }

    #[test]
    fn agent_detail_reads_the_script_as_json() {
        let home = scratch("ok");
        std::fs::write(
            home.join("agents").join("a-1.js"),
            "window.TIRITH_AGENT = {\"id\":\"a-1\",\"report\":\"full <\\/p>\",\"log_entries\":[{\"kind\":\"say\"}]};\n",
        )
        .unwrap();
        let d = agent_detail_in(&home, "a-1").unwrap();
        assert_eq!(d["id"], "a-1");
        assert_eq!(d["report"], "full </p>");
        assert_eq!(d["log_entries"][0]["kind"], "say");
        assert!(agent_detail_in(&home, "a-2").is_err());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn agent_detail_cannot_leave_the_agents_folder() {
        let home = scratch("escape");
        std::fs::write(home.join("secret.js"), "window.TIRITH_AGENT = {\"id\":\"x\"};").unwrap();
        for id in ["../secret", "..", "/etc/passwd", "a/b", ""] {
            assert!(!agent_file_name(id).contains('/'), "{id}");
            assert!(agent_detail_in(&home, id).is_err(), "{id}");
        }
        assert_eq!(agent_file_name("a-1"), "a-1.js");
        assert_eq!(agent_file_name(".hidden"), "_hidden.js");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn broken_detail_file_is_an_error() {
        let home = scratch("bad");
        std::fs::write(home.join("agents").join("a-1.js"), "alert(1)").unwrap();
        assert!(agent_detail_in(&home, "a-1").is_err());
        std::fs::write(home.join("agents").join("a-2.js"), "window.TIRITH_AGENT = {oops};").unwrap();
        assert!(agent_detail_in(&home, "a-2").is_err());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn pump_pushes_only_when_the_time_and_the_content_changed() {
        let mut pump = Pump::default();
        let a = json!({"generated_at": "2026-10-29T10:00:00Z", "agents": [1]});
        assert_eq!(pump.decide(&a), Push::State);
        assert_eq!(pump.decide(&a), Push::Nothing, "same file read again");
        let tick = json!({"generated_at": "2026-10-29T10:00:02Z", "agents": [1]});
        assert_eq!(pump.decide(&tick), Push::Alive("2026-10-29T10:00:02Z".into()), "only the clock moved");
        let changed = json!({"generated_at": "2026-10-29T10:00:04Z", "agents": [1, 2]});
        assert_eq!(pump.decide(&changed), Push::State);
        let same_time_other_content = json!({"generated_at": "2026-10-29T10:00:04Z", "agents": [9]});
        assert_eq!(pump.decide(&same_time_other_content), Push::Nothing);
    }
}

// -- collector ----------------------------------------------------------------------------------

#[cfg(unix)]
extern "C" {
    fn flock(fd: std::os::raw::c_int, operation: std::os::raw::c_int) -> std::os::raw::c_int;
}

/// True when something holds the advisory lock `tirith collect` keeps on `<home>/collector.lock`.
/// We try to take it without waiting and let go again at once; a missing file means no collector.
#[cfg(unix)]
fn collector_lock_held(home: &Path) -> bool {
    use std::os::unix::io::AsRawFd;
    const LOCK_EX: std::os::raw::c_int = 2;
    const LOCK_NB: std::os::raw::c_int = 4;
    let Ok(file) = std::fs::File::open(home.join("collector.lock")) else {
        return false;
    };
    // SAFETY: `file` owns a valid descriptor for the whole call; closing it on drop releases our lock.
    unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) != 0 }
}

/// No lock check on Windows: the app always starts its own (the collector itself refuses to double-run
/// only where it can take the lock).
#[cfg(not(unix))]
fn collector_lock_held(_home: &Path) -> bool {
    false
}

fn collector_running() -> bool {
    collector_lock_held(&tirith_home())
}

#[cfg(all(test, unix))]
mod collector_lock_tests {
    use super::*;
    use std::os::unix::io::AsRawFd;

    #[test]
    fn lock_check_follows_the_collector_lock() {
        let home = std::env::temp_dir().join(format!("tirith-clock-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        assert!(!collector_lock_held(&home), "no lock file yet");

        let file = std::fs::File::create(home.join("collector.lock")).unwrap();
        assert!(!collector_lock_held(&home), "file exists but nobody holds it");

        assert_eq!(unsafe { flock(file.as_raw_fd(), 2 | 4) }, 0);
        assert!(collector_lock_held(&home), "held by a collector");
        assert!(collector_lock_held(&home), "checking must not release or steal it");

        drop(file);
        assert!(!collector_lock_held(&home), "released when the holder exits");
        let _ = std::fs::remove_dir_all(&home);
    }
}

fn start_collector(app: &App) {
    if collector_running() {
        return;
    }
    let (out, err) = collector_log().unwrap_or_else(|| (Stdio::null(), Stdio::null()));
    match tirith_command(&["collect", "--interval", "2"]).stdin(Stdio::null()).stdout(out).stderr(err).spawn() {
        Ok(child) => *app.collector.lock().unwrap() = Some(child),
        Err(e) => {
            eprintln!("tirith: could not start the collector: {e}");
            log_line("app.log", &format!("could not start the collector: {e}"));
        }
    }
}

fn stop_collector(app: &App) {
    if let Some(mut child) = app.collector.lock().unwrap().take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

// -- launch requests ----------------------------------------------------------------------------

/// Runs `tirith launch --project P --step N` and returns what it printed.
#[tauri::command]
fn launch_step(project: String, step: u32) -> Result<String, String> {
    let out = tirith_command(&["launch", "--project", &project, "--step", &step.to_string()])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("could not run tirith: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status.success() {
        Ok(text)
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() { text } else { err })
    }
}

// -- add a project -----------------------------------------------------------------------------

/// Folders under a temp directory are scratch space, never a project.
fn in_temp_dir(folder: &std::path::Path) -> bool {
    let mut roots: Vec<PathBuf> = ["/tmp", "/var/tmp", "/private/tmp", "/private/var/tmp", "/private/var/folders"]
        .iter()
        .map(PathBuf::from)
        .collect();
    roots.push(std::env::temp_dir());
    roots
        .iter()
        .filter_map(|r| r.canonicalize().ok().or_else(|| Some(r.clone())))
        .any(|r| folder.starts_with(r))
}

/// The real folder to register, or why it cannot be one.
fn validate_project_folder(folder: &str) -> Result<PathBuf, String> {
    if folder.trim().is_empty() {
        return Err("pick a folder".into());
    }
    let path = expand(folder.trim());
    let real = path.canonicalize().map_err(|_| format!("folder not found: {folder}"))?;
    if !real.is_dir() {
        return Err(format!("not a folder: {folder}"));
    }
    if in_temp_dir(&real) {
        return Err("a temporary folder cannot be a project".into());
    }
    Ok(real)
}

/// `#rrggbb` or nothing (the collector picks a color).
fn validate_project_color(color: &str) -> Result<Option<String>, String> {
    let c = color.trim();
    if c.is_empty() {
        return Ok(None);
    }
    let ok = c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit());
    if ok { Ok(Some(c.to_string())) } else { Err(format!("color must look like #2f7d5b, got {c:?}")) }
}

/// The argv for `tirith project add`; every value is one argument, never a shell string.
fn project_add_args(folder: &std::path::Path, name: &str, color: &str) -> Result<Vec<String>, String> {
    let name = name.trim();
    if name.chars().count() > 80 || name.chars().any(|c| c.is_control()) {
        return Err("the name must be one line of at most 80 characters".into());
    }
    let mut args = vec!["project".to_string(), "add".to_string(), folder.to_string_lossy().into_owned()];
    if !name.is_empty() {
        args.push(format!("--name={name}"));
    }
    if let Some(c) = validate_project_color(color)? {
        args.push(format!("--color={c}"));
    }
    Ok(args)
}

/// Registers a folder with `tirith project add` and reports the new project and whether the folder
/// already has a `tirith.toml`.
#[tauri::command]
fn project_add(folder: String, name: String, color: String) -> Result<Value, String> {
    let real = validate_project_folder(&folder)?;
    let args = project_add_args(&real, &name, &color)?;
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = tirith_command(&argv)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("could not run tirith: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() { text } else { err });
    }
    // "<id>: <folder>" when created, "<folder> is already registered as <id>" otherwise
    let created = !text.contains(" is already registered as ");
    let id = match text.rsplit_once(" is already registered as ") {
        Some((_, id)) => id.trim().to_string(),
        None => text.split(':').next().unwrap_or("").trim().to_string(),
    };
    Ok(json!({
        "id": id,
        "folder": real.to_string_lossy(),
        "created": created,
        "hasToml": real.join("tirith.toml").is_file(),
        "message": text,
    }))
}

#[cfg(test)]
mod project_add_tests {
    use super::*;

    fn scratch_in_home(tag: &str) -> PathBuf {
        let dir = user_home().join(format!(".tirith-test-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn rejects_missing_empty_and_file_paths() {
        assert!(validate_project_folder("").is_err());
        assert!(validate_project_folder("/definitely/not/here-4821").is_err());
        let file = std::env::current_exe().unwrap();
        assert!(validate_project_folder(file.to_str().unwrap()).unwrap_err().contains("not a folder"));
    }

    #[test]
    fn rejects_temp_folders_even_through_symlinks() {
        let tmp = std::env::temp_dir().join(format!("tirith-pa-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(validate_project_folder(tmp.to_str().unwrap()).unwrap_err().contains("temporary"));
        assert!(validate_project_folder("/tmp").is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn accepts_a_real_folder_outside_temp() {
        let dir = scratch_in_home("ok");
        let got = validate_project_folder(dir.to_str().unwrap());
        let _ = std::fs::remove_dir_all(&dir);
        if in_temp_dir(&user_home()) {
            return; // a sandboxed HOME under a temp folder: nothing to assert
        }
        assert_eq!(got.unwrap().file_name(), dir.file_name());
    }

    #[test]
    fn color_is_a_hex_triplet_or_empty() {
        assert_eq!(validate_project_color("").unwrap(), None);
        assert_eq!(validate_project_color(" #2f7d5b ").unwrap().as_deref(), Some("#2f7d5b"));
        assert!(validate_project_color("red").is_err());
        assert!(validate_project_color("#12345").is_err());
        assert!(validate_project_color("#12345g").is_err());
    }

    #[test]
    fn argv_keeps_every_value_a_single_argument() {
        let folder = PathBuf::from("/Users/me/my project; rm -rf ~");
        let args = project_add_args(&folder, "  --evil name ", "#112233").unwrap();
        assert_eq!(args, vec!["project", "add", "/Users/me/my project; rm -rf ~", "--name=--evil name", "--color=#112233"]);
        let bare = project_add_args(&folder, "", "").unwrap();
        assert_eq!(bare.len(), 3);
        assert!(project_add_args(&folder, "two\nlines", "").is_err());
        assert!(project_add_args(&folder, &"x".repeat(81), "").is_err());
        assert!(project_add_args(&folder, "ok", "nope").is_err());
    }
}

// -- inbox requests ----------------------------------------------------------------------------

const INBOX_TEXT_MAX: usize = 4096;

/// UTC `YYYY-MM-DDTHH:MM:SSZ` for a Unix time (days-from-civil, no date crate).
fn iso_utc(secs: u64) -> String {
    let (days, rem) = ((secs / 86400) as i64, secs % 86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// `text` cut to at most `max` bytes, never inside a character.
fn clip(text: &str, max: usize) -> &str {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

const ATTACH_MAX: usize = 6;
const ATTACH_MAX_BYTES: usize = 10 * 1024 * 1024;

/// Decodes standard base64 (padding optional, whitespace ignored).
fn b64_decode(input: &str) -> Result<Vec<u8>, String> {
    let (mut out, mut acc, mut bits) = (Vec::with_capacity(input.len() / 4 * 3), 0u32, 0u32);
    for c in input.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=') {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err("the image is not valid base64".into()),
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

/// `(mime type, extension)` of a PNG, JPEG, GIF or WebP from its first bytes.
fn sniff_image(data: &[u8]) -> Option<(&'static str, &'static str)> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("image/png", "png"))
    } else if data.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(("image/jpeg", "jpg"))
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        Some(("image/gif", "gif"))
    } else if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        Some(("image/webp", "webp"))
    } else {
        None
    }
}

/// A message id is `in-` and 8 lowercase hex digits: the only shape allowed to name a folder.
fn valid_message_id(id: &str) -> bool {
    id.len() == 11 && id.starts_with("in-") && id[3..].bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

/// Writes one image to `<root>/<message_id>/<n>.<ext>`; the type comes from the bytes (never the name),
/// the folder from the validated id, so nothing is written outside `root`.
/// Returns `{path, type, size}` (the page adds width and height).
fn attachment_write(root: &std::path::Path, message_id: &str, name: &str, data: &[u8]) -> Result<Value, String> {
    if !valid_message_id(message_id) {
        return Err(format!("bad message id {message_id:?}"));
    }
    if data.is_empty() || data.len() > ATTACH_MAX_BYTES {
        return Err(format!("{name}: an image must be between 1 byte and 10 MB"));
    }
    let (mime, ext) = sniff_image(data).ok_or_else(|| format!("{name}: only PNG, JPEG, GIF and WebP images are accepted"))?;
    let dir = root.join(message_id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let taken = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
    if taken >= ATTACH_MAX {
        return Err(format!("at most {ATTACH_MAX} images per message"));
    }
    let path = dir.join(format!("{}.{ext}", taken + 1));
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .and_then(|mut f| f.write_all(data))
        .map_err(|e| format!("could not write {}: {e}", path.display()))?;
    Ok(json!({"path": path.to_string_lossy(), "type": mime, "size": data.len()}))
}

/// Saves an image the owner attached to a chat message under `~/.tirith/attachments/<message_id>/`
/// and returns `{path, type, size}`; call it before `inbox_add` with the same id.
#[tauri::command]
fn attachment_save(message_id: String, name: String, bytes_base64: String) -> Result<Value, String> {
    if bytes_base64.len() > ATTACH_MAX_BYTES / 3 * 4 + 8 {
        return Err(format!("{name}: an image must be at most 10 MB"));
    }
    attachment_write(&tirith_home().join("attachments"), &message_id, &name, &b64_decode(&bytes_base64)?)
}

/// The attachments of a request as stored: at most 6, each `{path, type, size, width, height}` with the path
/// inside `root` and no `..`; anything else is refused.
fn clean_attachments(raw: &[Value], root: &std::path::Path) -> Result<Vec<Value>, String> {
    if raw.len() > ATTACH_MAX {
        return Err(format!("at most {ATTACH_MAX} images per message"));
    }
    raw.iter()
        .map(|a| {
            let path = a["path"].as_str().unwrap_or("");
            let p = std::path::Path::new(path);
            let inside = p.starts_with(root) && !p.components().any(|c| matches!(c, std::path::Component::ParentDir));
            let mime = a["type"].as_str().unwrap_or("");
            if !inside || !["image/png", "image/jpeg", "image/gif", "image/webp"].contains(&mime) {
                return Err(format!("bad attachment {path:?}"));
            }
            let num = |k: &str| a[k].as_u64().unwrap_or(0);
            Ok(json!({"path": path, "type": mime, "size": num("size"), "width": num("width"), "height": num("height")}))
        })
        .collect()
}

/// Builds the inbox row for a request: `{id, at, kind, project, step, text, from, status}`.
fn inbox_row(kind: &str, project: &str, step: &str, text: &str, now: SystemTime, salt: u32) -> Result<Value, String> {
    let plan = kind == "plan-refresh" || kind == "plan-new";
    if kind != "finish" && kind != "ask" && kind != "setup" && kind != "chat" && kind != "autopilot" && kind != "plan-edit" && !plan {
        return Err(format!("kind must be finish, ask, setup, chat, autopilot, plan-refresh, plan-new or plan-edit, not {kind:?}"));
    }
    let (project, text) = (project.trim(), clip(text.trim(), INBOX_TEXT_MAX));
    let step = if step.trim().is_empty() && kind == "setup" {
        "setup"
    } else if step.trim().is_empty() && plan {
        "plan"
    } else {
        step.trim()
    };
    if kind == "chat" {
        if text.is_empty() {
            return Err("a chat message needs text".into());
        }
    } else if project.is_empty() || step.is_empty() {
        return Err("a request needs a project and a step".into());
    }
    if kind == "ask" && text.is_empty() {
        return Err("a question needs text".into());
    }
    if kind == "autopilot" && text != "on" && text != "off" {
        return Err("an autopilot request needs text on or off".into());
    }
    if (kind == "setup" || kind == "plan-new" || kind == "plan-edit") && !text.starts_with('{') {
        return Err(format!("a {kind} request needs its JSON as text"));
    }
    let d = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let id = format!("in-{:08x}", (d.as_nanos() as u32) ^ salt.rotate_left(16) ^ std::process::id());
    Ok(json!({
        "id": id, "at": iso_utc(d.as_secs()), "kind": kind, "project": project, "step": step,
        "text": text, "from": "owner", "status": "open",
    }))
}

/// Appends one row to `<home>/inbox.jsonl` in a single write call, so a concurrent writer cannot split the line.
fn inbox_append(row: &Value) -> Result<(), String> {
    inbox_append_in(&tirith_home(), row)
}

fn inbox_append_in(home: &std::path::Path, row: &Value) -> Result<(), String> {
    std::fs::create_dir_all(&home).map_err(|e| format!("could not create {}: {e}", home.display()))?;
    let path = home.join("inbox.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("could not open {}: {e}", path.display()))?;
    file.write_all(format!("{row}\n").as_bytes()).map_err(|e| e.to_string())
}

/// A client id as the page makes it: 1 to 64 characters of `[A-Za-z0-9_-]`.
fn valid_cid(cid: &str) -> bool {
    !cid.is_empty() && cid.len() <= 64 && cid.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

/// The id of the request row of `inbox.jsonl` that already carries this client id, if any.
fn id_of_cid(home: &std::path::Path, cid: &str) -> Option<String> {
    let text = std::fs::read_to_string(home.join("inbox.jsonl")).ok()?;
    text.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|r| r["cid"] == cid && r["id"].is_string() && r["kind"] != "answer")
        .and_then(|r| r["id"].as_str().map(String::from))
}

/// Appends `<time> <line>` to `<home>/logs/chat-send.log`; a log that cannot be written never fails a send.
fn chat_send_log(home: &std::path::Path, line: &str) {
    let dir = home.join("logs");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("chat-send.log")) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let _ = file.write_all(format!("{} {line}\n", iso_utc(now)).as_bytes());
    }
}

fn inbox_salt() -> u32 {
    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Appends one request to `<home>/inbox.jsonl` (no shell) and returns its id. A `cid` already on a row is a
/// retry: nothing is written and that row's id comes back. Every attempt and its result go to `logs/chat-send.log`.
#[tauri::command]
fn inbox_add(
    kind: String,
    project: String,
    step: String,
    text: Option<String>,
    id: Option<String>,
    attachments: Option<Vec<Value>>,
    cid: Option<String>,
) -> Result<String, String> {
    inbox_add_in(&tirith_home(), kind, project, step, text, id, attachments, cid)
}

#[allow(clippy::too_many_arguments)]
fn inbox_add_in(
    home: &std::path::Path,
    kind: String,
    project: String,
    step: String,
    text: Option<String>,
    id: Option<String>,
    attachments: Option<Vec<Value>>,
    cid: Option<String>,
) -> Result<String, String> {
    let head = format!(
        "send kind={kind} project={project:?} cid={} chars={} images={}",
        cid.as_deref().unwrap_or("-"),
        text.as_deref().map_or(0, |t| t.chars().count()),
        attachments.as_ref().map_or(0, |a| a.len())
    );
    let result = inbox_add_row(home, &kind, &project, &step, text, id, attachments, cid);
    match &result {
        Ok(id) => chat_send_log(home, &format!("{head} -> ok id={id}")),
        Err(e) => chat_send_log(home, &format!("{head} -> error: {e}")),
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn inbox_add_row(
    home: &std::path::Path,
    kind: &str,
    project: &str,
    step: &str,
    text: Option<String>,
    id: Option<String>,
    attachments: Option<Vec<Value>>,
    cid: Option<String>,
) -> Result<String, String> {
    let cid = cid.filter(|c| !c.is_empty());
    if let Some(c) = &cid {
        if !valid_cid(c) {
            return Err(format!("bad client id {c:?}"));
        }
        if let Some(seen) = id_of_cid(home, c) {
            return Ok(seen);
        }
    }
    let attachments = clean_attachments(&attachments.unwrap_or_default(), &home.join("attachments"))?;
    let text = text.unwrap_or_default();
    if !attachments.is_empty() && kind != "chat" {
        return Err("only a chat message takes images".into());
    }
    // a chat message may be only images: the row builder wants text, the stored row keeps it empty
    let probe = if attachments.is_empty() || !text.trim().is_empty() { text.as_str() } else { "(image)" };
    let mut row = inbox_row(kind, project, step, probe, SystemTime::now(), inbox_salt())?;
    row["text"] = json!(clip(text.trim(), INBOX_TEXT_MAX));
    if let Some(id) = id {
        if !valid_message_id(&id) {
            return Err(format!("bad message id {id:?}"));
        }
        row["id"] = json!(id);
    }
    if let Some(c) = cid {
        row["cid"] = json!(c);
    }
    if !attachments.is_empty() {
        row["attachments"] = json!(attachments);
    }
    inbox_append_in(home, &row)?;
    Ok(row["id"].as_str().unwrap_or_default().to_string())
}

/// Builds the inbox row for the owner's answer to an ask: `{id, at, kind: "reply", wait, project, value, text, from, status}`.
/// `value` is the option chosen and `text` the free words; one of them is required.
fn reply_row(wait: &str, project: &str, value: &str, text: &str, now: SystemTime, salt: u32) -> Result<Value, String> {
    let (wait, project, value, text) = (wait.trim(), project.trim(), clip(value.trim(), 200), clip(text.trim(), INBOX_TEXT_MAX));
    if wait.is_empty() {
        return Err("a reply needs the id of the ask".into());
    }
    if value.is_empty() && text.is_empty() {
        return Err("a reply needs a value or a text".into());
    }
    let d = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let id = format!("in-{:08x}", (d.as_nanos() as u32) ^ salt.rotate_left(16) ^ std::process::id());
    Ok(json!({
        "id": id, "at": iso_utc(d.as_secs()), "kind": "reply", "wait": wait, "project": project,
        "value": value, "text": text, "from": "owner", "status": "open",
    }))
}

/// Appends the owner's answer to an ask to `<home>/inbox.jsonl` (the manager watches it) and returns its id.
#[tauri::command]
fn inbox_reply(wait: String, project: Option<String>, value: Option<String>, text: Option<String>) -> Result<String, String> {
    let row = reply_row(
        &wait,
        project.as_deref().unwrap_or(""),
        value.as_deref().unwrap_or(""),
        text.as_deref().unwrap_or(""),
        SystemTime::now(),
        inbox_salt(),
    )?;
    inbox_append(&row)?;
    Ok(row["id"].as_str().unwrap_or_default().to_string())
}

#[cfg(test)]
mod inbox_tests {
    use super::*;

    #[test]
    fn iso_utc_known_dates() {
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_utc(1_782_734_400), "2026-06-29T12:00:00Z");
        assert_eq!(iso_utc(1_709_164_800), "2024-02-29T00:00:00Z");
    }

    #[test]
    fn row_has_the_documented_fields() {
        let row = inbox_row("ask", " demo ", "2", " why? ", UNIX_EPOCH + Duration::from_secs(1_782_734_400), 1).unwrap();
        assert_eq!(row["kind"], "ask");
        assert_eq!(row["project"], "demo");
        assert_eq!(row["text"], "why?");
        assert_eq!(row["from"], "owner");
        assert_eq!(row["status"], "open");
        assert!(row["id"].as_str().unwrap().starts_with("in-"));
    }

    #[test]
    fn rejects_bad_requests_and_clips_text() {
        assert!(inbox_row("ask", "demo", "1", "", UNIX_EPOCH, 0).is_err());
        assert!(inbox_row("nope", "demo", "1", "x", UNIX_EPOCH, 0).is_err());
        assert!(inbox_row("finish", "", "1", "", UNIX_EPOCH, 0).is_err());
        assert!(inbox_row("finish", "demo", "1", "", UNIX_EPOCH, 0).is_ok());
        assert!(inbox_row("setup", "demo", "", "{\"action\":\"add-stage\"}", UNIX_EPOCH, 0).is_ok());
        assert!(inbox_row("setup", "demo", "", "add a stage", UNIX_EPOCH, 0).is_err());
        assert_eq!(inbox_row("plan-refresh", "demo", "", "", UNIX_EPOCH, 0).unwrap()["step"], "plan");
        assert!(inbox_row("plan-new", "demo", "", "{\"title\":\"x\"}", UNIX_EPOCH, 0).is_ok());
        assert!(inbox_row("plan-new", "demo", "", "a title", UNIX_EPOCH, 0).is_err());
        assert!(inbox_row("plan-edit", "demo", "2", "{\"add\":\"x\"}", UNIX_EPOCH, 0).is_ok());
        assert!(inbox_row("plan-edit", "demo", "2", "x", UNIX_EPOCH, 0).is_err());
        let long = "é".repeat(3000);
        let row = inbox_row("ask", "demo", "1", &long, UNIX_EPOCH, 0).unwrap();
        assert!(row["text"].as_str().unwrap().len() <= INBOX_TEXT_MAX);
    }

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tirith-att-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn base64_decodes_with_and_without_padding() {
        assert_eq!(b64_decode("aGk=").unwrap(), b"hi");
        assert_eq!(b64_decode("aGk").unwrap(), b"hi");
        assert_eq!(b64_decode("aGVs\nbG8=").unwrap(), b"hello");
        assert!(b64_decode("a*b").is_err());
    }

    #[test]
    fn attachment_is_saved_by_its_bytes_under_the_message_folder() {
        let root = scratch("ok");
        let a = attachment_write(&root, "in-0a1b2c3d", "../../evil.sh", PNG).unwrap();
        assert_eq!(a["type"], "image/png");
        assert_eq!(a["size"], PNG.len());
        let path = PathBuf::from(a["path"].as_str().unwrap());
        assert_eq!(path, root.join("in-0a1b2c3d").join("1.png"));
        assert_eq!(std::fs::read(&path).unwrap(), PNG);
        let b = attachment_write(&root, "in-0a1b2c3d", "x.png", &[0xff, 0xd8, 0xff, 0xe0]).unwrap();
        assert!(b["path"].as_str().unwrap().ends_with("/2.jpg"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn attachment_validation_refuses_bad_ids_types_and_sizes() {
        let root = scratch("bad");
        for id in ["", "../x", "in-../..", "in-ABCDEF01", "in-0a1b2c3", "x/in-0a1b2c3d", "in-0a1b2c3d/.."] {
            assert!(attachment_write(&root, id, "a.png", PNG).is_err(), "{id}");
        }
        assert!(attachment_write(&root, "in-0a1b2c3d", "a.png", b"<svg/>").is_err());
        assert!(attachment_write(&root, "in-0a1b2c3d", "a.png", b"").is_err());
        let big = [PNG, &vec![0u8; ATTACH_MAX_BYTES][..]].concat();
        assert!(attachment_write(&root, "in-0a1b2c3d", "a.png", &big).is_err());
        assert!(!root.exists());
        for _ in 0..ATTACH_MAX {
            attachment_write(&root, "in-0a1b2c3d", "a.png", PNG).unwrap();
        }
        assert!(attachment_write(&root, "in-0a1b2c3d", "a.png", PNG).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn stored_attachments_must_live_inside_the_attachments_folder() {
        let root = PathBuf::from("/h/attachments");
        let ok = json!({"path": "/h/attachments/in-0a1b2c3d/1.png", "type": "image/png", "size": 9, "width": 4, "height": 3});
        assert_eq!(clean_attachments(&[ok.clone()], &root).unwrap()[0]["width"], 4);
        for path in ["/etc/passwd", "/h/attachments/../secret.png", "/h/attachmentsX/1.png", "rel.png"] {
            let bad = json!({"path": path, "type": "image/png"});
            assert!(clean_attachments(&[bad], &root).is_err(), "{path}");
        }
        assert!(clean_attachments(&[json!({"path": "/h/attachments/a/1.svg", "type": "image/svg+xml"})], &root).is_err());
        assert!(clean_attachments(&vec![ok; 7], &root).is_err());
    }

    #[test]
    fn a_retry_with_the_same_cid_writes_one_row_and_every_attempt_is_logged() {
        let home = scratch("cid");
        let add = |cid: &str| inbox_add_in(&home, "chat".into(), "demo".into(), "".into(), Some("hello".into()), None, None, Some(cid.into()));
        let first = add("c-1").unwrap();
        assert_eq!(add("c-1").unwrap(), first);
        assert_ne!(add("c-2").unwrap(), first);
        let rows = std::fs::read_to_string(home.join("inbox.jsonl")).unwrap();
        assert_eq!(rows.lines().count(), 2);
        assert!(rows.lines().next().unwrap().contains("\"cid\":\"c-1\""));
        let log = std::fs::read_to_string(home.join("logs/chat-send.log")).unwrap();
        assert_eq!(log.lines().count(), 3);
        assert!(log.contains("cid=c-1") && log.contains("-> ok id="));
        assert!(!log.contains("hello"), "the log never holds the message text");
    }

    #[test]
    fn a_failed_send_writes_no_row_and_logs_the_error() {
        let home = scratch("fail");
        let bad = inbox_add_in(&home, "chat".into(), "demo".into(), "".into(), Some("  ".into()), None, None, Some("c-9".into()));
        assert!(bad.is_err());
        let bad_cid = inbox_add_in(&home, "chat".into(), "".into(), "".into(), Some("x".into()), None, None, Some("bad cid!".into()));
        assert!(bad_cid.is_err());
        assert!(!home.join("inbox.jsonl").exists());
        let log = std::fs::read_to_string(home.join("logs/chat-send.log")).unwrap();
        assert_eq!(log.matches("-> error:").count(), 2);
    }

    #[test]
    fn reply_row_has_the_documented_fields() {
        let row = reply_row(" w-7 ", " demo ", " ship ", " but later ", UNIX_EPOCH + Duration::from_secs(1_782_734_400), 2).unwrap();
        assert_eq!(row["kind"], "reply");
        assert_eq!(row["wait"], "w-7");
        assert_eq!(row["project"], "demo");
        assert_eq!(row["value"], "ship");
        assert_eq!(row["text"], "but later");
        assert_eq!(row["from"], "owner");
        assert_eq!(row["status"], "open");
        assert_eq!(row["at"], "2026-06-29T12:00:00Z");
        assert!(row["id"].as_str().unwrap().starts_with("in-"));
    }

    #[test]
    fn reply_needs_an_ask_and_something_to_say() {
        assert!(reply_row("", "demo", "x", "", UNIX_EPOCH, 0).is_err());
        assert!(reply_row("w-1", "demo", " ", " ", UNIX_EPOCH, 0).is_err());
        assert!(reply_row("w-1", "", "", "free words", UNIX_EPOCH, 0).is_ok());
        assert!(reply_row("w-1", "", "go", "", UNIX_EPOCH, 0).is_ok());
    }

    #[test]
    fn reply_text_is_clipped_on_a_character_boundary() {
        let long = "é".repeat(3000);
        let row = reply_row("w-1", "demo", "", &long, UNIX_EPOCH, 0).unwrap();
        assert!(row["text"].as_str().unwrap().len() <= INBOX_TEXT_MAX);
        let value = "ñ".repeat(300);
        let row = reply_row("w-1", "demo", &value, "", UNIX_EPOCH, 0).unwrap();
        assert!(row["value"].as_str().unwrap().len() <= 200);
    }
}

// -- actions: the owner's "Run" buttons ----------------------------------------------------------
//
// The page sends an action id and nothing else: the command is looked up in `inbox.jsonl` (answers)
// and `waits.jsonl`, run with `/bin/sh -c`, and its output goes to `<home>/actions/<id>.log`.
// See docs/CONTRACT-actions.md.

const ACTION_STALE_S: u64 = 3600;

/// Every JSON object of a JSONL file (bad lines skipped).
fn read_jsonl(path: &std::path::Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .map(|t| t.lines().filter_map(|l| serde_json::from_str::<Value>(l).ok()).collect())
        .unwrap_or_default()
}

/// `(action, project)` for an action id: from an answer row of `inbox.jsonl`, else a wait of `waits.jsonl`.
fn find_action(home: &std::path::Path, action_id: &str) -> Option<(Value, String)> {
    let inbox = read_jsonl(&home.join("inbox.jsonl"));
    let has = |row: &Value| -> Option<Value> {
        row.get("actions")?.as_array()?.iter().find(|a| a["id"] == action_id).cloned()
    };
    for row in inbox.iter().filter(|r| r["kind"] == "answer") {
        if let Some(action) = has(row) {
            let project = inbox
                .iter()
                .find(|r| r["id"] == row["id"] && (r["kind"] == "ask" || r["kind"] == "finish"))
                .and_then(|r| r["project"].as_str())
                .unwrap_or("");
            return Some((action, project.to_string()));
        }
    }
    let mut waits: HashMap<String, Value> = HashMap::new();
    for row in read_jsonl(&home.join("waits.jsonl")) {
        if let Some(id) = row["id"].as_str() {
            waits.insert(id.to_string(), row);
        }
    }
    if let Some(found) = waits.values().find_map(|row| {
        has(row).map(|a| (a, row["project"].as_str().unwrap_or("").to_string()))
    }) {
        return Some(found);
    }
    // actions the collector itself makes (guard checks): `generated-actions.json`, id -> action + project
    let generated: Value = std::fs::read_to_string(home.join("generated-actions.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())?;
    let action = generated.get(action_id)?.clone();
    let project = action["project"].as_str().unwrap_or("").to_string();
    Some((action, project))
}

/// Unix seconds of `YYYY-MM-DDTHH:MM:SSZ`.
fn parse_iso_utc(text: &str) -> Option<u64> {
    let b = text.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[10] != b'T' {
        return None;
    }
    let n = |r: std::ops::Range<usize>| text.get(r)?.parse::<i64>().ok();
    let (y, m, d) = (n(0..4)?, n(5..7)?, n(8..10)?);
    let (hh, mm, ss) = (n(11..13)?, n(14..16)?, n(17..19)?);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    u64::try_from(days * 86400 + hh * 3600 + mm * 60 + ss).ok()
}

/// True when the latest `action-run` row of the action says `running` and is not stale.
fn disk_running(home: &std::path::Path, action_id: &str, now: u64) -> bool {
    let rows = read_jsonl(&home.join("inbox.jsonl"));
    let last = rows.iter().rev().find(|r| r["kind"] == "action-run" && r["action_id"] == action_id);
    match last {
        Some(r) if r["status"] == "running" => {
            let at = r["at"].as_str().and_then(parse_iso_utc);
            at.map(|t| now.saturating_sub(t) < ACTION_STALE_S).unwrap_or(true)
        }
        _ => false,
    }
}

/// Appends `{kind: "action-run", action_id, status, exit_code, at}` to `inbox.jsonl` (one write).
fn append_run_row(home: &std::path::Path, action_id: &str, status: &str, exit_code: Option<i32>) -> Result<(), String> {
    let at = iso_utc(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs());
    let row = json!({ "kind": "action-run", "action_id": action_id, "status": status, "exit_code": exit_code, "at": at });
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(home.join("inbox.jsonl"))
        .map_err(|e| e.to_string())?;
    file.write_all(format!("{row}\n").as_bytes()).map_err(|e| e.to_string())
}

/// The folder an action runs in: its `cwd`, else the project's folder, else the user's home.
fn action_cwd(action: &Value, project: &str, home: &std::path::Path) -> PathBuf {
    if let Some(cwd) = action["cwd"].as_str().filter(|c| !c.is_empty()) {
        return expand(cwd);
    }
    if let Some(repo) = project_repo(&config(), project) {
        return repo;
    }
    let auto = std::fs::read_to_string(home.join("projects-auto.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok());
    auto.and_then(|a| {
        a["projects"].as_array()?.iter().find(|p| p["id"] == project)?["folder"].as_str().map(expand)
    })
    .unwrap_or_else(user_home)
}

/// Runs `command` with `/bin/sh -c` in `cwd`, output (both streams) into `log`. Returns the exit code
/// (127 when it could not start).
fn run_shell(command: &str, cwd: &std::path::Path, log: &std::path::Path) -> i32 {
    let started = std::fs::File::create(log).and_then(|out| {
        let err = out.try_clone()?;
        let mut c = Command::new("/bin/sh");
        with_child_path(&mut c);
        c.arg("-c").arg(command).current_dir(cwd).stdin(Stdio::null()).stdout(out).stderr(err).status()
    });
    match started {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            let _ = std::fs::write(log, format!("tirith: {e}\n"));
            127
        }
    }
}

/// Runs a declared action: `running` row now, `passed` or `failed` when the command ends.
/// Returns at once; the page follows the rows through the state.
/// True when `state` lists `lane_id` as an agent that is still running (working or stalled) and is a
/// local lane (remote work has no process of ours to stop).
fn lane_is_running(state: &Value, lane_id: &str) -> bool {
    state["agents"].as_array().map_or(false, |agents| {
        agents.iter().any(|a| {
            a["id"] == lane_id
                && (a["status"] == "working" || a["status"] == "stalled")
                && a["source"] != "github"
        })
    })
}

/// Stops a running lane for the owner: `tirith lane stop <id> --by owner`, in the background (it waits up
/// to 10 s for the agent to exit). The id is checked against the lanes in `state.json`, never run as typed.
#[tauri::command]
fn lane_stop(lane_id: String) -> Result<(), String> {
    let lane_id = lane_id.trim().to_string();
    let state = read_state().ok_or("no state yet")?;
    if !lane_is_running(&state, &lane_id) {
        return Err(format!("{lane_id} is not a running lane"));
    }
    let child = tirith_command(&["lane", "stop", &lane_id, "--by", "owner"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not run tirith: {e}"))?;
    std::thread::spawn(move || {
        let mut child = child;
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(test)]
mod lane_stop_tests {
    use super::*;

    #[test]
    fn only_running_local_lanes_can_be_stopped() {
        let state = json!({"agents": [
            {"id": "a", "status": "working"}, {"id": "b", "status": "stalled"},
            {"id": "c", "status": "done"}, {"id": "d", "status": "working", "source": "github"}]});
        assert!(lane_is_running(&state, "a"));
        assert!(lane_is_running(&state, "b"));
        assert!(!lane_is_running(&state, "c"));
        assert!(!lane_is_running(&state, "d"));
        assert!(!lane_is_running(&state, "x; rm -rf ~"));
        assert!(!lane_is_running(&json!({}), "a"));
    }
}

#[tauri::command]
fn run_action(app: AppHandle, action_id: String) -> Result<(), String> {
    let home = tirith_home();
    let (action, project) = find_action(&home, &action_id).ok_or_else(|| format!("no action {action_id}"))?;
    let command = action["command"].as_str().filter(|c| !c.trim().is_empty()).ok_or("the action has no command")?.to_string();
    let cwd = action_cwd(&action, &project, &home);
    if !cwd.is_dir() {
        return Err(format!("the folder {} does not exist", cwd.display()));
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let app_state = app.state::<App>();
    if !app_state.running.lock().unwrap().insert(action_id.clone()) {
        return Err(format!("action {action_id} is already running"));
    }
    let started = if disk_running(&home, &action_id, now) {
        Err(format!("action {action_id} is already running"))
    } else {
        let dir = home.join("actions");
        std::fs::create_dir_all(&dir)
            .map_err(|e| e.to_string())
            .and_then(|_| append_run_row(&home, &action_id, "running", None))
            .map(|_| dir)
    };
    let dir = match started {
        Ok(dir) => dir,
        Err(e) => {
            app_state.running.lock().unwrap().remove(&action_id);
            return Err(e);
        }
    };
    std::thread::spawn(move || {
        let code = run_shell(&rewrite_tirith(&command, &tirith_cmd()), &cwd, &dir.join(format!("{action_id}.log")));
        let _ = append_run_row(&home, &action_id, if code == 0 { "passed" } else { "failed" }, Some(code));
        app.state::<App>().running.lock().unwrap().remove(&action_id);
    });
    Ok(())
}

#[cfg(test)]
mod action_tests {
    use super::*;

    fn temp_home(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tirith-actions-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_lines(path: PathBuf, rows: &[Value]) {
        let text: String = rows.iter().map(|r| format!("{r}\n")).collect();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn finds_actions_in_answers_and_waits_only_by_id() {
        let home = temp_home("find");
        let a = json!({"id": "ac-1", "label": "L", "command": "echo hi", "cwd": null});
        let w = json!({"id": "ac-2", "label": "W", "command": "true"});
        write_lines(home.join("inbox.jsonl"), &[
            json!({"id": "in-1", "kind": "ask", "project": "demo", "step": "1", "text": "q"}),
            json!({"id": "in-1", "kind": "answer", "by": "manager", "text": "t", "actions": [a]}),
        ]);
        write_lines(home.join("waits.jsonl"), &[json!({"id": "w-1", "project": "other", "actions": [w]})]);
        let (found, project) = find_action(&home, "ac-1").unwrap();
        assert_eq!((found["command"].as_str(), project.as_str()), (Some("echo hi"), "demo"));
        assert_eq!(find_action(&home, "ac-2").unwrap().1, "other");
        assert!(find_action(&home, "echo hi").is_none());
        assert!(find_action(&home, "ac-9").is_none());
    }

    #[test]
    fn bare_tirith_becomes_the_resolved_command() {
        let c = "/r/bin/tirith";
        assert_eq!(rewrite_tirith("tirith shim install", c), "/r/bin/tirith shim install");
        assert_eq!(rewrite_tirith("cd x && tirith url install; tirith check", c), "cd x && /r/bin/tirith url install; /r/bin/tirith check");
        assert_eq!(rewrite_tirith("echo tirith\ntirith a", c), "echo tirith\n/r/bin/tirith a");
        for same in ["echo tirith", "/r/bin/tirith x", "tirithx y", "git commit -m 'tirith: x'"] {
            assert_eq!(rewrite_tirith(same, c), same);
        }
    }

    #[test]
    fn the_resolved_command_is_absolute_and_quoted() {
        let cmd = tirith_cmd();
        assert!(cmd.starts_with('/') || cmd.starts_with("uv ") || cmd.starts_with('\''), "{cmd}");
        assert_eq!(sh_quote("a b"), "'a b'");
    }

    #[test]
    fn iso_roundtrip() {
        for secs in [0u64, 1_782_734_400, 1_709_164_800] {
            assert_eq!(parse_iso_utc(&iso_utc(secs)), Some(secs));
        }
        assert_eq!(parse_iso_utc("nope"), None);
    }

    #[test]
    fn running_row_blocks_until_stale_or_finished() {
        let home = temp_home("running");
        let now = 1_782_734_400;
        let at = iso_utc(now - 60);
        write_lines(home.join("inbox.jsonl"), &[
            json!({"kind": "action-run", "action_id": "ac-1", "status": "running", "exit_code": null, "at": at}),
        ]);
        assert!(disk_running(&home, "ac-1", now));
        assert!(!disk_running(&home, "ac-1", now + ACTION_STALE_S + 120));
        assert!(!disk_running(&home, "ac-2", now));
        append_run_row(&home, "ac-1", "passed", Some(0)).unwrap();
        assert!(!disk_running(&home, "ac-1", now));
        let rows = read_jsonl(&home.join("inbox.jsonl"));
        assert_eq!(rows.last().unwrap()["status"], "passed");
        assert_eq!(rows.last().unwrap()["exit_code"], 0);
    }

    #[test]
    fn shell_runs_in_the_folder_and_logs_both_streams() {
        let home = temp_home("shell");
        let log = home.join("a.log");
        assert_eq!(run_shell("pwd; echo out; echo err >&2", &home, &log), 0);
        let text = std::fs::read_to_string(&log).unwrap();
        assert!(text.contains("out") && text.contains("err") && text.contains("tirith-actions-shell"));
        assert_eq!(run_shell("exit 3", &home, &log), 3);
        assert_eq!(run_shell("true", &home.join("missing"), &log), 127);
    }

    #[test]
    fn cwd_prefers_the_action_then_falls_back() {
        let home = temp_home("cwd");
        let with = json!({"cwd": "/tmp/x"});
        assert_eq!(action_cwd(&with, "demo", &home), PathBuf::from("/tmp/x"));
        let none = json!({"cwd": null});
        assert!(!action_cwd(&none, "no-such-project", &home).as_os_str().is_empty());
    }
}

// -- chat (behind a flag) -----------------------------------------------------------------------
//
// TODO: resuming a conversation and showing tool calls. Text and permission questions only.

/// Chat is off unless `TIRITH_CHAT=1` or `[app] chat = true` in `config.toml`.
#[tauri::command]
fn chat_enabled() -> bool {
    if std::env::var("TIRITH_CHAT").map(|v| v == "1").unwrap_or(false) {
        return true;
    }
    config().get("app").and_then(|a| a.get("chat")).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Aliases from `[agents.<alias>]` that define `chat_args` (the streaming JSON mode arguments).
#[tauri::command]
fn chat_agents() -> Vec<String> {
    let cfg = config();
    let mut v: Vec<String> = cfg
        .get("agents")
        .and_then(|a| a.as_table())
        .map(|t| {
            t.iter()
                .filter(|(_, a)| a.get("command").is_some() && a.get("chat_args").is_some())
                .map(|(k, _)| k.clone())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

fn emit_chat(app: &AppHandle, id: &str, kind: &str, line: &str) {
    let _ = app.emit(CHAT_EVENT, json!({ "id": id, "kind": kind, "line": line }));
}

/// Starts a headless agent session in the project's repo as a lane (`lane run --interactive`).
/// Returns the session id (also the lane id).
#[tauri::command]
fn chat_start(
    app: AppHandle,
    state: State<'_, App>,
    project: String,
    alias: String,
) -> Result<String, String> {
    if !chat_enabled() {
        return Err("chat is off (set TIRITH_CHAT=1 or [app] chat = true in config.toml)".into());
    }
    let cfg = config();
    let agent = cfg
        .get("agents")
        .and_then(|a| a.get(alias.as_str()))
        .ok_or_else(|| format!("no [agents.{alias}] in config.toml"))?;
    let command = agent
        .get("command")
        .and_then(|c| c.as_str())
        .ok_or_else(|| format!("[agents.{alias}] has no command"))?;
    let args: Vec<String> = agent
        .get("chat_args")
        .and_then(|a| a.as_array())
        .ok_or_else(|| format!("[agents.{alias}] has no chat_args (its streaming JSON mode arguments)"))?
        .iter()
        .filter_map(|a| a.as_str().map(str::to_string))
        .collect();

    // The conversation is a lane: `lane run --interactive` registers it, runs the agent on our pipes
    // (it inherits stdin/stdout), forwards signals and ends the lane however the agent exits.
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let id = format!("chat-{ms}");
    let brief = tirith_home().join("chats").join(format!("{id}.md"));
    std::fs::create_dir_all(brief.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&brief, format!("Chat with {alias}: the owner talks to the agent from the app.\n"))
        .map_err(|e| e.to_string())?;
    let mut cmd = tirith_command(&[
        "lane", "run", "--interactive", "--project", &project, "--class", "chat", "--id", &id,
        "--feature", &format!("Chat with {alias}"), "--brief", &brief.to_string_lossy(), "--",
    ]);
    cmd.arg(command).args(&args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("could not start {alias}: {e}"))?;
    let stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;
    state.chats.lock().unwrap().insert(id.clone(), ChatSession { child, stdin });

    let (a, i) = (app.clone(), id.clone());
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            emit_chat(&a, &i, line_kind(&line), &line);
        }
        // stdout closed: `lane run` is done and has ended the lane. Reap it.
        let code = a
            .state::<App>()
            .chats
            .lock()
            .unwrap()
            .remove(&i)
            .and_then(|mut s| s.child.wait().ok())
            .and_then(|s| s.code());
        emit_chat(&a, &i, "exit", if code == Some(0) || code.is_none() { "done" } else { "failed" });
    });
    let (a, i) = (app.clone(), id.clone());
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            emit_chat(&a, &i, "err", &line);
        }
    });
    Ok(id)
}

/// Agent output is one JSON object per line; anything else is `lane run`'s own text (its summary).
fn line_kind(line: &str) -> &'static str {
    if line.trim_start().starts_with('{') { "out" } else { "sys" }
}

/// One user turn in the agent's streaming JSON input mode: `type: user`, content as text blocks.
fn user_message(text: &str) -> Value {
    json!({ "type": "user", "message": { "role": "user", "content": [{ "type": "text", "text": text }] } })
}

/// Sends one user message as a line of streaming JSON input.
#[tauri::command]
fn chat_send(state: State<'_, App>, id: String, text: String) -> Result<(), String> {
    let mut chats = state.chats.lock().unwrap();
    let s = chats.get_mut(&id).ok_or("this chat has ended")?;
    let msg = user_message(&text);
    writeln!(s.stdin, "{msg}").and_then(|_| s.stdin.flush()).map_err(|e| e.to_string())
}

/// The reply to an agent's permission question (a `control_request` line with subtype
/// `can_use_tool`). `input` is the tool input the agent asked about, sent back unchanged on allow.
fn permission_reply(request_id: &str, allow: bool, input: Value) -> Value {
    let response = if allow {
        json!({ "behavior": "allow", "updatedInput": input })
    } else {
        json!({ "behavior": "deny", "message": "The owner denied this in the Tirith app." })
    };
    json!({ "type": "control_response",
            "response": { "subtype": "success", "request_id": request_id, "response": response } })
}

/// Answers a permission question the agent asked on its stdout.
#[tauri::command]
fn chat_answer(
    state: State<'_, App>,
    id: String,
    request_id: String,
    allow: bool,
    input: Option<Value>,
) -> Result<(), String> {
    let mut chats = state.chats.lock().unwrap();
    let s = chats.get_mut(&id).ok_or("this chat has ended")?;
    let msg = permission_reply(&request_id, allow, input.unwrap_or_else(|| json!({})));
    writeln!(s.stdin, "{msg}").and_then(|_| s.stdin.flush()).map_err(|e| e.to_string())
}

/// Asks `lane run` to stop: it forwards the signal to the agent and ends the lane itself.
fn terminate(child: &mut Child) {
    #[cfg(unix)]
    {
        let ok = Command::new("kill").args(["-TERM", &child.id().to_string()]).status();
        if ok.map(|s| s.success()).unwrap_or(false) {
            return;
        }
    }
    let _ = child.kill();
}

/// Ends a chat; the reader thread reports the exit.
#[tauri::command]
fn chat_stop(state: State<'_, App>, id: String) {
    if let Some(s) = state.chats.lock().unwrap().get_mut(&id) {
        terminate(&mut s.child);
    }
}

// -- entry --------------------------------------------------------------------------------------

pub fn run() {
    let live = web_dir(&config());
    let serve = live.clone();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(App::default())
        .register_uri_scheme_protocol(liveweb::SCHEME, move |_ctx, req| {
            let (code, ty, body) = match &serve {
                Some(root) => liveweb::respond(root, req.uri().path()),
                None => (404, "text/plain; charset=utf-8", b"live web is off".to_vec()),
            };
            tauri::http::Response::builder()
                .status(code)
                .header("Content-Type", ty)
                .header("Cache-Control", "no-store")
                .body(body)
                .unwrap()
        })
        .invoke_handler(tauri::generate_handler![
            current_state,
            agent_detail,
            launch_step,
            project_add,
            inbox_add,
            attachment_save,
            inbox_reply,
            run_action,
            lane_stop,
            chat_enabled,
            chat_agents,
            chat_start,
            chat_send,
            chat_answer,
            chat_stop
        ])
        .setup(move |app| {
            let url = match &live {
                Some(root) => {
                    spawn_web_watch(app.handle().clone(), root.clone());
                    WebviewUrl::External(liveweb::page_url().parse().expect("live page url"))
                }
                None => WebviewUrl::App("index.html".into()),
            };
            WebviewWindowBuilder::new(app, "main", url)
                .title("Tirith")
                .inner_size(1440.0, 900.0)
                .build()?;
            start_collector(&app.state::<App>());
            spawn_state_pump(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the Tirith app");

    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            let state = handle.state::<App>();
            for (_, mut s) in state.chats.lock().unwrap().drain() {
                terminate(&mut s.child);
                let _ = s.child.wait();
            }
            stop_collector(&state);
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn child_path_prepends_tool_folders_once() {
        let home = std::path::Path::new("/home/u");
        let p = super::child_path(Some("/opt/x/bin/uv"), home, Some(std::ffi::OsStr::new("/usr/bin:/bin:/usr/local/bin")));
        let got: Vec<String> = std::env::split_paths(&p).map(|d| d.to_string_lossy().into_owned()).collect();
        assert_eq!(
            got,
            ["/opt/x/bin", "/opt/homebrew/bin", "/usr/local/bin", "/home/u/.local/bin", "/home/u/.cargo/bin", "/usr/bin", "/bin"]
        );
        let bare = super::child_path(Some("uv"), home, None);
        assert_eq!(std::env::split_paths(&bare).next().unwrap().to_string_lossy(), "/opt/homebrew/bin");
    }

    use super::*;

    #[test]
    fn permission_reply_allow_echoes_the_input() {
        let r = permission_reply("r1", true, json!({ "command": "ls" }));
        assert_eq!(r["type"], "control_response");
        assert_eq!(r["response"]["request_id"], "r1");
        assert_eq!(r["response"]["response"]["behavior"], "allow");
        assert_eq!(r["response"]["response"]["updatedInput"]["command"], "ls");
    }

    #[test]
    fn permission_reply_deny_carries_a_message() {
        let r = permission_reply("r2", false, json!({}));
        assert_eq!(r["response"]["response"]["behavior"], "deny");
        assert!(r["response"]["response"]["message"].as_str().unwrap().len() > 3);
    }

    #[test]
    fn user_message_is_a_text_block_turn() {
        let m = user_message("reply with ok");
        assert_eq!(m["type"], "user");
        assert_eq!(m["message"]["role"], "user");
        assert_eq!(m["message"]["content"][0]["type"], "text");
        assert_eq!(m["message"]["content"][0]["text"], "reply with ok");
    }

    #[test]
    fn lane_text_is_not_agent_output() {
        assert_eq!(line_kind("{\"type\":\"assistant\"}"), "out");
        assert_eq!(line_kind("lane chat-1: done, 0 files +0 -0, 3s"), "sys");
    }
}
