//! What Codex's own process writes in its state folder, found by running the real CLI in empty ones: no settings, `.env`, profiles, rules, prompts, plugins or skills, which steer a later run.
use super::sb::Sb;

const FOLDERS: [&str; 8] = [
    "sessions",
    "archived_sessions",
    "log",
    "logs",
    "shell_snapshots",
    "thread-writer-locks",
    "tmp/arg0",
    "tmp/arg0-macos",
];
const FILES: [&str; 8] = [
    "tmp",
    "installation_id",
    "models_cache.json",
    "history.jsonl",
    "session_index.jsonl",
    "version.json",
    ".sandbox_migration",
    ".personality_migration",
];

/// Its sign-in, which it renews in the file that is there: none can be planted.
const RENEWED: &str = "auth.json";

/// Those, for the program `own` names alone.
pub(super) fn own(sb: &mut Sb, state: &str, own: &str) {
    let folder = sb.path(state);
    let mut what = vec![format!("(literal {folder})")]; // a first run makes it
    what.push(format!(
        "(require-all (subpath {folder}) (regex #\"/[^/]+[.]sqlite(-wal|-shm|-journal)?$\"))"
    ));
    for name in FOLDERS {
        what.push(format!("(subpath {})", sb.path(&format!("{state}/{name}"))));
    }
    for name in FILES {
        what.push(format!("(literal {})", sb.path(&format!("{state}/{name}"))));
    }
    sb.line(&format!(
        "(allow file-write* (require-all {own} (require-any {})))",
        what.join(" ")
    ));
    let renewed = sb.path(&format!("{state}/{RENEWED}"));
    sb.line(&format!(
        "(allow file-write-data file-write-times (require-all {own} (literal {renewed})))"
    ));
}
