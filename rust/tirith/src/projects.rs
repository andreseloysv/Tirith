//! Adding a project (the page's "+", `tirith project add`): the folder gets a git repository, the commit guard and a
//! plan with a first step, then a `[[projects]]` block appended to config.toml (its existing text is never rewritten).

use crate::app::{bad, git, Shared, R};
use crate::config::{expand, home_dir, Project};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

const COLORS: [&str; 8] = ["#2f7d5b", "#3b6fd8", "#8a5cd0", "#d0663b", "#c2417c", "#b8860b", "#0e8aa0", "#64748b"];
/// One config.toml write at a time, so two adds never lose one another; one folder picker on screen at a time.
static WRITING: std::sync::Mutex<()> = std::sync::Mutex::new(());
static PICKING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
/// AppKit's open panel through JXA, floating above every window: since macOS 14 a helper process may not come to the
/// front, so AppleScript's `choose folder` opened behind the page. It prints the folder, or nothing when cancelled.
const PANEL: &str = "ObjC.import('AppKit'); function run(argv) { const app = $.NSApplication.sharedApplication, p = $.NSOpenPanel.openPanel; \
    app.setActivationPolicy($.NSApplicationActivationPolicyAccessory); p.canChooseFiles = false; p.canChooseDirectories = true; p.canCreateDirectories = true; \
    p.message = 'Choose the project folder'; p.directoryURL = $.NSURL.fileURLWithPath(argv[0]); p.level = $.NSStatusWindowLevel; app.activateIgnoringOtherApps(true); \
    return p.runModal == $.NSModalResponseOK ? ObjC.unwrap(p.URL.path) : '' }";

/// The projects as the page shows them.
pub fn list(app: &Shared) -> Vec<Value> {
    app.cfg().projects.iter().map(|p| json!({"id": p.id, "name": p.name, "color": p.color, "autopilot": crate::lanes::autopilot_on(app, &p.id), "main_red": app.kv(&format!("main_red:{}", p.id))})).collect()
}

/// The real folder behind `raw` (a full path or `~/…`), or why it cannot be a project; a project's own folder passes.
fn folder(raw: &str, projects: &[Project], tirith: &Path) -> Result<PathBuf, String> {
    let path = if raw.trim() == "~" { home_dir() } else { expand(raw.trim()) };
    let missing = if path.is_absolute() { format!("there is no folder {raw:?}") } else { format!("{raw:?} is not the full path of a folder (/Users/… or ~/…)") };
    let dir = path.canonicalize().ok().filter(|d| path.is_absolute() && d.is_dir()).ok_or(missing)?;
    let temp = ["/tmp", "/private/tmp", "/var/tmp", "/private/var/tmp", "/var/folders", "/private/var/folders"].map(PathBuf::from).into_iter().chain([std::env::temp_dir()]);
    let why = if temp.map(|t| t.canonicalize().unwrap_or(t)).any(|t| dir.starts_with(t)) {
        Some("is a temporary folder".to_string())
    } else if home_dir().canonicalize().unwrap_or_else(|_| home_dir()).starts_with(&dir) {
        Some("holds your home folder".into())
    } else if dir.components().any(|c| c.as_os_str() == ".git") || dir.join(".git").is_file() {
        Some("is a git worktree or inside a .git folder".into())
    } else if dir.starts_with(tirith.canonicalize().unwrap_or_else(|_| tirith.into())) {
        Some("is Tirith's own folder (the agents' copies of the projects live there)".into())
    } else {
        let named = |p: &Project, what: &str| format!("{what} {}", if p.name.is_empty() { &p.id } else { &p.name });
        projects.iter().filter(|p| !p.repo.is_empty() && p.repo() != dir).find_map(|p| dir.starts_with(p.repo()).then(|| named(p, "is inside the project")).or_else(|| p.repo().starts_with(&dir).then(|| named(p, "holds the project"))))
    };
    why.map_or(Ok(()), |w| Err(format!("{} {w}: pick a project's own folder", dir.display())))?;
    Ok(dir)
}

/// The project as it will be written: `name` (else the folder's), `color` (else the first one no project has), an id.
fn project(dir: &Path, name: &str, color: &str, projects: &[Project]) -> Result<Project, String> {
    let name = Some(name.trim().to_string()).filter(|n| !n.is_empty()).or_else(|| Some(dir.file_name()?.to_string_lossy().into_owned())).unwrap_or_default();
    if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
        return Err("the name is one line of 1 to 80 characters".into());
    }
    let c = color.trim().to_lowercase();
    let free = COLORS.iter().find(|c| !projects.iter().any(|p| p.color.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(c)))).unwrap_or(&COLORS[projects.len() % COLORS.len()]);
    let color = match c.as_str() {
        "" => free.to_string(),
        _ if c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|h| h.is_ascii_hexdigit()) => c,
        _ => return Err(format!("the color must look like #2f7d5b, not {color:?}")),
    };
    Ok(Project { id: unique_id(&name, projects), name, repo: dir.display().to_string(), color: Some(color) })
}

/// `name` in lowercase letters, digits and dashes, with -2, -3… when that id is taken.
fn unique_id(name: &str, projects: &[Project]) -> String {
    let base = Some(crate::lanes::slug(name)).filter(|s| !s.is_empty()).unwrap_or_else(|| "project".into());
    (1..).map(|n| if n == 1 { base.clone() } else { format!("{base}-{n}") }).find(|id| !projects.iter().any(|p| p.id == *id)).unwrap_or(base)
}

/// What to append to config.toml (`text`) for the project, checked to parse with the project in it.
fn block(text: &str, p: &Project) -> Result<String, String> {
    let q = |s: &str| toml::Value::String(s.into()).to_string();
    let sep = if text.trim().is_empty() { "" } else { &"\n\n"[usize::from(text.ends_with('\n'))..] }; // one blank line before the block
    let block = format!("{sep}[[projects]]\nid = {}\nname = {}\nrepo = {}\ncolor = {}\n", q(&p.id), q(&p.name), q(&p.repo), q(p.color.as_deref().unwrap_or("")));
    let t: toml::Table = format!("{text}{block}").parse().map_err(|e| format!("config.toml would not parse with the project: {e}"))?;
    let added = crate::config::get::<Vec<Project>>(&t, &["projects"]).is_some_and(|ps| ps.iter().any(|x| x.id == p.id && x.repo == p.repo));
    added.then_some(block).ok_or("config.toml has a `projects` key that is not [[projects]] tables: add the project there by hand".into())
}

/// Appends the project to config.toml and reloads it. The project that has the folder (another add may have won).
fn register(app: &Shared, mut p: Project) -> Result<(Project, bool), String> {
    let _one = WRITING.lock().unwrap();
    app.reload_config(true);
    let cfg = app.cfg();
    if let Some(old) = cfg.projects.iter().find(|o| !o.repo.is_empty() && o.repo() == p.repo()) {
        return Ok((old.clone(), false));
    }
    p.id = unique_id(&p.name, &cfg.projects);
    let file = app.home.join("config.toml");
    let fail = |e: std::io::Error| format!("{}: {e}", file.display());
    let text = match std::fs::read_to_string(&file) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(fail(e)),
        r => r.unwrap_or_default(),
    };
    let more = block(&text, &p)?;
    std::fs::OpenOptions::new().create(true).append(true).mode(0o600).open(&file).and_then(|mut f| f.write_all(more.as_bytes())).map_err(fail)?;
    app.reload_config(true);
    Ok((p, true))
}

/// Adds the folder as a project; with `dry`, only says what adding it would do.
async fn add(app: &Shared, raw: &str, name: &str, color: &str, dry: bool) -> Result<Value, String> {
    let cfg = app.cfg();
    let dir = folder(raw, &cfg.projects, &app.home)?;
    let shown = |p: &Project, exists: bool, notes: &[String]| json!({"id": p.id, "name": p.name, "folder": dir, "color": p.color, "exists": exists, "notes": notes});
    if let Some(p) = cfg.projects.iter().find(|p| !p.repo.is_empty() && p.repo() == dir) {
        return Ok(shown(p, true, &[]));
    }
    // A repo git cannot read is an error, not a folder to init; a home folder kept in git is no project's repo.
    let home = home_dir().canonicalize().unwrap_or_else(|_| home_dir());
    let top = match git(&dir, &["rev-parse", "--show-toplevel"]).await {
        Err(e) if dir.join(".git").exists() => return Err(e),
        r => r.ok().map(|t| PathBuf::from(&t).canonicalize().unwrap_or_else(|_| t.into())).filter(|t| !home.starts_with(t)),
    };
    if let Some(t) = top.as_ref().filter(|t| **t != dir) {
        return Err(format!("{} is inside the git repository {}: add that folder instead", dir.display(), t.display()));
    }
    let p = project(&dir, name, color, &cfg.projects)?;
    block(&std::fs::read_to_string(app.home.join("config.toml")).unwrap_or_default(), &p)?; // config.toml takes it: only then touch the folder
    let plan = dir.join(app.manifest(&dir).roadmap);
    let open = std::fs::read_to_string(&plan).map(|t| crate::plan::parse(&t).iter().flat_map(|s| &s.items).filter(|i| !i.done).count()).unwrap_or(0);
    let start = !plan.exists() && plan.starts_with(&dir) && !plan.components().any(|c| c == Component::ParentDir); // never outside the folder
    if dry {
        return Ok(json!({"id": p.id, "name": p.name, "folder": dir, "color": p.color, "git": top.is_some(), "plan": !start, "open": open}));
    }
    let mut notes = vec![];
    if top.is_none() {
        git(&dir, &["init", "-q"]).await?;
        notes.push("Created a git repository in the folder.".to_string());
    }
    notes.extend(crate::guard::install(app, &p).err());
    if start {
        let rel = plan.strip_prefix(&dir).unwrap_or(&plan).display().to_string();
        let started: Result<String, String> = async {
            plan.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&plan, "# Plan\n\n## 1. First steps\n")).map_err(|e| e.to_string())?;
            git(&dir, &["-c", "core.fsmonitor=false", "add", "--", &rel]).await?;
            git(&dir, &["-c", "core.fsmonitor=false", "commit", "-q", "-m", "Plan: start the plan", "--", &rel]).await
        }
        .await;
        notes.push(started.map_or_else(|e| format!("Its plan {rel} could not be started: {e}"), |_| format!("Started its plan in {rel}: add the first items on the Overview.")));
    }
    // A plan that already has open items: agents start on it only once the owner resumes the autopilot.
    let hold = |id: &str| (open > 0).then(|| app.set_kv(&format!("autopilot:{id}"), Some("off")));
    hold(&p.id);
    let (p, created) = register(app, p)?;
    hold(&p.id);
    notes.extend((open > 0).then(|| format!("Its plan has {open} open items: the autopilot waits until you resume it on the Overview.")));
    app.emit("projects", json!(list(app)));
    app.sched_wake.notify_one();
    Ok(shown(&p, !created, &notes))
}

/// Git repositories next to the projects or in the home folder that are no project yet, the most recently used first.
async fn folders(State(app): State<Shared>) -> Json<Value> {
    let repos: Vec<PathBuf> = app.cfg().projects.iter().filter(|p| !p.repo.is_empty()).map(Project::repo).collect();
    let mut roots: Vec<PathBuf> = repos.iter().filter_map(|r| r.parent().map(Path::to_path_buf)).chain([home_dir()]).collect();
    roots.sort();
    roots.dedup();
    let entries = roots.iter().flat_map(|r| std::fs::read_dir(r).into_iter().flatten().flatten()).map(|e| e.path());
    let used = |d: PathBuf| Some((std::fs::metadata(d.join(".git")).ok().filter(|m| m.is_dir())?.modified().ok()?, d));
    let mut found: Vec<_> = entries.filter(|d| !repos.contains(d) && !d.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'))).filter_map(used).collect();
    found.sort_by_key(|f| std::cmp::Reverse(f.0));
    Json(json!(found.iter().take(200).map(|(_, d)| json!({"folder": d, "name": d.file_name().map(|n| n.to_string_lossy())})).collect::<Vec<_>>()))
}

/// The macOS folder picker, opened by the daemon in the owner's session (one at a time): the folder, or "" when cancelled.
async fn pick(State(app): State<Shared>) -> R {
    let Ok(_one) = PICKING.try_lock() else { return Err(bad("the folder picker is already open".into())) };
    let start = app.cfg().projects.iter().find(|p| !p.repo.is_empty()).and_then(|p| p.repo().parent().map(Path::to_path_buf)).filter(|d| d.is_dir()).unwrap_or_else(home_dir);
    let out = tokio::process::Command::new("osascript").args(["-l", "JavaScript", "-e", PANEL]).arg(start).kill_on_drop(true).output();
    let out = tokio::time::timeout(Duration::from_secs(600), out).await.map_err(|_| bad("the folder picker was open too long".into()))?.map_err(|e| bad(format!("no folder picker here ({e}): type the path")))?;
    if !out.status.success() {
        return Err(bad(format!("the folder picker did not open: {}", String::from_utf8_lossy(&out.stderr).trim())));
    }
    Ok(Json(json!({"folder": String::from_utf8_lossy(&out.stdout).trim()})))
}

pub fn routes() -> Router<Shared> {
    let add_route = |State(app): State<Shared>, Json(b): Json<Value>| async move {
        let s = |k: &str| crate::server::s(&b, k);
        add(&app, &s("folder"), &s("name"), &s("color"), b["dry"] == true).await.map(Json).map_err(bad)
    };
    Router::new().route("/api/projects", post(add_route)).route("/api/projects/folders", get(folders)).route("/api/projects/pick", post(pick))
}

/// `tirith project add <folder> [--name N] [--color #rrggbb]`.
pub fn cli(pos: &[&str], flag: crate::client::Flag) -> Option<Result<(), String>> {
    let ["project", "add", raw] = pos else { return None };
    if std::env::var("TIRITH_LANE").is_ok_and(|l| !l.is_empty()) {
        return Some(Err("a lane does not add projects: ask the manager with tirith ask \"<question>\"".into()));
    }
    let full = std::env::current_dir().map(|d| d.join(raw).display().to_string()).unwrap_or(raw.to_string()); // an absolute path stays
    let body = json!({"folder": if raw.starts_with('~') { raw.to_string() } else { full }, "name": flag("--name").unwrap_or_default(), "color": flag("--color").unwrap_or_default()});
    Some(crate::client::call("POST", "/api/projects", Some(body)).and_then(|b| serde_json::from_str::<Value>(&b).map_err(|e| e.to_string())).map(|v| {
        let (id, dir) = (v["id"].as_str().unwrap_or(""), v["folder"].as_str().unwrap_or(""));
        println!("{}", if v["exists"] == true { format!("{dir} is already the project {id}") } else { format!("{id}: {dir}") });
        v["notes"].as_array().into_iter().flatten().filter_map(Value::as_str).for_each(|n| println!("  {n}"));
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_add_appends_a_block_and_keeps_the_file() {
        let old = "# mine\n[[projects]]\nid = \"a\"\nrepo = \"~/a\"\n\n[guard]\nwords = [\"x\"]";
        let p = project(Path::new("/Users/me/my \"b\""), "Quote \" and \\ back", "", &[]).unwrap();
        let ps: Vec<Project> = crate::config::get(&format!("{old}{}", block(old, &p).unwrap()).parse::<toml::Table>().unwrap(), &["projects"]).unwrap();
        assert_eq!((ps.len(), ps[1].id.as_str(), ps[1].name.as_str(), ps[1].repo.as_str()), (2, "quote-and-back", "Quote \" and \\ back", "/Users/me/my \"b\""));
        assert!(block("projects = []\n", &p).is_err() && block("x = [\n", &p).is_err() && block("", &p).unwrap().starts_with("[[projects]]"));
    }
    #[test]
    fn project_add_names_ids_and_colors() {
        let mk = |id: &str| Project { id: id.into(), color: Some(COLORS[0].into()), ..Default::default() };
        let ps = [mk("my-app"), mk("my-app-2")];
        let p = |name: &str, color: &str| project(Path::new("/a/zoo"), name, color, &ps);
        assert_eq!((p("My App", "").unwrap().id, p("", "").unwrap().name, p("¿?", "").unwrap().id), ("my-app-3".into(), "zoo".into(), "project".into()));
        assert_eq!((p("x", "").unwrap().color, p("x", " #ABCDEF ").unwrap().color), (Some(COLORS[1].into()), Some("#abcdef".into())));
        assert!(["red", "#12345g", "#ñññ"].iter().all(|c| p("x", c).is_err()) && p("two\nlines", "").is_err() && p(&"x".repeat(81), "").is_err());
    }
    #[test]
    fn project_add_refuses_folders_that_cannot_be_projects() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../target/projects-test-{}", std::process::id()));
        let at = |p: &str| base.join(p).display().to_string();
        for d in ["repo/sub", "repo/.git/info", "home/lanes/repo/slot-1", "wt", "other"] {
            std::fs::create_dir_all(base.join(d)).unwrap();
        }
        std::fs::write(base.join("file"), "x").and_then(|_| std::fs::write(base.join("wt/.git"), "gitdir: /elsewhere")).unwrap();
        let (ps, home) = ([Project { id: "repo".into(), repo: at("repo"), ..Default::default() }], base.join("home"));
        let refusals = [("", "full path"), ("relative/dir", "full path"), ("/no/such/dir-4821", "no folder"), ("/tmp", "temporary"), ("/", "home folder")];
        let mine = [("file", "no folder"), ("repo/sub", "inside the project"), ("home/lanes/repo/slot-1", "Tirith's own"), ("home", "Tirith's own"), ("", "holds the project"), ("repo/.git/info", ".git"), ("wt", "worktree")];
        let err = |raw: String| folder(&raw, &ps, &home).err().unwrap_or_default();
        let got: Vec<(String, &str)> = refusals.iter().map(|(r, w)| (err(r.to_string()), *w)).chain(mine.iter().map(|(r, w)| (err(at(r)), *w))).collect();
        let fine = (folder(&at("other"), &ps, &home), folder(&at("repo"), &ps, &home));
        let _ = std::fs::remove_dir_all(&base);
        assert!(got.iter().all(|(e, w)| e.contains(w)), "{got:?}");
        assert!(fine.0.is_ok() && fine.1.is_ok());
    }
}
