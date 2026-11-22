//! How old a note of a bot's memory is, in runs: the bot's folder is a git repository, and the commit that last wrote the note says when.
use crate::app::Shared;
use std::path::Path;
use std::process::Stdio;

/// When the last commit that touched the line of note `id` (`m3`) in `memory.md` was made, as a UTC time written like the turns' (`2026-11-21T08:00:00Z`); none when the note is not in the history (not committed yet, no repository).
fn written_at(dir: &Path, id: &str) -> Option<String> {
    let pick = format!("^[[:space:]]*[-*] \\[{id}\\]");
    let out = crate::sandbox::git()
        .arg("-C")
        .arg(dir)
        .args(["log", "-1", "--format=%ct", "-G", &pick, "--", "memory.md"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let secs: i64 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
    let at = chrono::DateTime::from_timestamp(secs, 0)?;
    Some(at.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

/// How many of the bot's own runs started after note `id` was written; none when it is not known when.
pub fn runs_since(app: &Shared, key: &str, dir: &Path, id: &str) -> Option<usize> {
    let at = written_at(dir, id)?;
    let sql = "SELECT COUNT(*) FROM turns WHERE conv = ?1 AND role = ?1 AND COALESCE(variant, '') <> 'B' AND started_at > ?2 AND NOT (COALESCE(status, '') = 'failed' AND COALESCE(why, '') <> '' AND ms IS NULL)";
    let n = app.query(sql, [key, at.as_str()], |r| r.get::<_, i64>(0));
    n.first().map(|n| *n as usize)
}
