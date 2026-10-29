//! Tirith desktop app: the web UI in `web/` plus a Rust side that replaces the state polling with
//! Tauri events, keeps a collector running, launches steps and hosts the (flagged) chat.
//! No servers and no open ports: everything between the page and Rust is Tauri IPC.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};

const STATE_EVENT: &str = "tirith://state";
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
}

// -- paths and config ---------------------------------------------------------------------------

fn expand(p: &str) -> PathBuf {
    match p.strip_prefix("~/") {
        Some(rest) => user_home().join(rest),
        None => PathBuf::from(p),
    }
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

/// `uv run --project <collector> tirith <args>`, ready to spawn.
fn tirith_command(args: &[&str]) -> Command {
    let cfg = config();
    let uv = app_setting(&cfg, "uv").unwrap_or_else(|| "uv".into());
    let mut c = Command::new(uv);
    c.arg("run").arg("--project").arg(collector_dir(&cfg)).arg("tirith").args(args);
    c.env("TIRITH_HOME", tirith_home());
    c
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

fn spawn_state_pump(app: AppHandle) {
    std::thread::spawn(move || loop {
        if let Some(s) = read_state() {
            let _ = app.emit(STATE_EVENT, s);
        }
        std::thread::sleep(Duration::from_secs(2));
    });
}

// -- collector ----------------------------------------------------------------------------------

#[cfg(unix)]
fn collector_running() -> bool {
    Command::new("pgrep")
        .args(["-f", "tirith collect"])
        .stdout(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// No cheap process search without extra dependencies: Windows always starts its own.
#[cfg(not(unix))]
fn collector_running() -> bool {
    false
}

fn start_collector(app: &App) {
    if collector_running() {
        return;
    }
    match tirith_command(&["collect", "--interval", "2"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => *app.collector.lock().unwrap() = Some(child),
        Err(e) => eprintln!("tirith: could not start the collector: {e}"),
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

// -- chat (behind a flag) -----------------------------------------------------------------------
//
// TODO: permission questions from the agent in the UI, resuming a conversation, and showing tool
// calls. This first version streams text only.

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

fn project_repo(cfg: &toml::Table, project: &str) -> Option<PathBuf> {
    cfg.get("projects")?
        .as_array()?
        .iter()
        .find(|p| p.get("id").and_then(|i| i.as_str()) == Some(project))?
        .get("repo")?
        .as_str()
        .map(expand)
}

fn emit_chat(app: &AppHandle, id: &str, kind: &str, line: &str) {
    let _ = app.emit(CHAT_EVENT, json!({ "id": id, "kind": kind, "line": line }));
}

/// Starts a headless agent session in the project's repo and registers it as a lane.
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

    let mut cmd = Command::new(command);
    cmd.args(&args)
        .env("TIRITH_HOME", tirith_home())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(repo) = project_repo(&cfg, &project) {
        cmd.current_dir(repo);
    }
    let mut child = cmd.spawn().map_err(|e| format!("could not start {alias}: {e}"))?;
    let stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;

    let ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let id = format!("chat-{ms}");
    let pid = child.id().to_string();
    // The session is a Tirith lane for as long as the process lives.
    let _ = tirith_command(&[
        "lane", "start", "--project", &project, "--class", "chat", "--id", &id, "--pid", &pid,
        "--feature", &format!("Chat with {alias}"),
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .status();

    state.chats.lock().unwrap().insert(id.clone(), ChatSession { child, stdin });

    let (a, i) = (app.clone(), id.clone());
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            emit_chat(&a, &i, "out", &line);
        }
        // stdout closed: the process is done (or closing). Reap it and end the lane.
        let code = a
            .state::<App>()
            .chats
            .lock()
            .unwrap()
            .remove(&i)
            .and_then(|mut s| s.child.wait().ok())
            .and_then(|s| s.code());
        let result = if code == Some(0) || code.is_none() { "done" } else { "failed" };
        let _ = tirith_command(&["lane", "end", &i, "--result", result])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        emit_chat(&a, &i, "exit", result);
    });
    let (a, i) = (app.clone(), id.clone());
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            emit_chat(&a, &i, "err", &line);
        }
    });
    Ok(id)
}

/// Sends one user message as a line of streaming JSON input.
#[tauri::command]
fn chat_send(state: State<'_, App>, id: String, text: String) -> Result<(), String> {
    let mut chats = state.chats.lock().unwrap();
    let s = chats.get_mut(&id).ok_or("this chat has ended")?;
    let msg = json!({ "type": "user", "message": { "role": "user", "content": text } });
    writeln!(s.stdin, "{msg}").and_then(|_| s.stdin.flush()).map_err(|e| e.to_string())
}

/// Ends a chat: the process is killed and the reader thread closes the lane.
#[tauri::command]
fn chat_stop(state: State<'_, App>, id: String) {
    if let Some(s) = state.chats.lock().unwrap().get_mut(&id) {
        let _ = s.child.kill();
    }
}

// -- entry --------------------------------------------------------------------------------------

pub fn run() {
    let app = tauri::Builder::default()
        .manage(App::default())
        .invoke_handler(tauri::generate_handler![
            current_state,
            launch_step,
            chat_enabled,
            chat_agents,
            chat_start,
            chat_send,
            chat_stop
        ])
        .setup(|app| {
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
                let _ = s.child.kill();
            }
            stop_collector(&state);
        }
    });
}
