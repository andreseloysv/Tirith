//! The bots' folders as git repositories: each bot's (`<home>/manager/<key>`) follows only its memory.md and its own skills (its `.gitignore` leaves out the links to the shared ones), and `<home>/bots` (the shared memory and skills) is another. Tirith commits each turn that changed them («turn <id>») and each change from the Library («library»), so the versions a turn keeps (git blob hashes, turns/versions.rs) are in this history.
use crate::app::App;
use std::path::Path;
use std::process::Stdio;
use std::sync::Mutex;

/// One commit at a time: the shared folder is every bot's.
static COMMITTING: Mutex<()> = Mutex::new(());

/// A bot's folder: its memory and the skill folders in `.agents/skills`; a link there (a shared skill) is no folder to git, so it stays out.
const OWN: &str = "# Tirith versions only the memory, the routines' own effort and this bot's own skills; the links to the shared skills\n# (in <home>/bots, a repository of its own) and everything else stay out.\n*\n!/.gitignore\n!/memory.md\n!/routines.toml\n!/.agents/\n/.agents/*\n!/.agents/skills/\n/.agents/skills/*\n!/.agents/skills/*/\n!/.agents/skills/*/**\n";
/// The bots' shared folder: the shared memory and skills.
const SHARED: &str = "# Tirith versions only the shared memory and the shared skills.\n*\n!/.gitignore\n!/memory.md\n!/skills/\n!/skills/**\n";

/// `git -C dir args…` as Tirith (its own name, no signing): its stdout, if it succeeded.
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = crate::sandbox::git()
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=Tirith",
            "-c",
            "user.email=tirith@localhost",
        ])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Commits what changed in `dir` (a repository from now on, with `ignore` as its `.gitignore`) as `message`; nothing when nothing changed or the folder does not exist yet.
fn commit(dir: &Path, ignore: &str, message: &str) {
    let Some(()) = dir.is_dir().then_some(()) else {
        return;
    };
    let _ = (!dir.join(".git").exists()).then(|| git(dir, &["init", "-q"]));
    let file = dir.join(".gitignore");
    let _ = (std::fs::read_to_string(&file).ok().as_deref() != Some(ignore))
        .then(|| std::fs::write(&file, ignore));
    let changed = git(dir, &["add", "-A"])
        .and_then(|_| git(dir, &["status", "--porcelain"]))
        .is_some_and(|s| !s.trim().is_empty());
    let _ = changed.then(|| git(dir, &["commit", "-q", "-m", message]));
}

/// Commits the folder of the bot whose conversation is `key` and the shared one, each only if it changed.
pub fn versioned(app: &App, key: &str, message: &str) {
    let _one = COMMITTING.lock().unwrap_or_else(|e| e.into_inner());
    super::drafts::sweep(app, key); // never a draft among the skills the agents load, nor in their history
    commit(&super::folder(app, key), OWN, message);
    commit(&app.home.join("bots"), SHARED, message);
}
