//! Muse's own log of a session, for a turn whose output carried no usage: the `model_completed` records of `~/.local/share/muse/sessions/<year>/<month>/<day>/<id>/session.jsonl`.
use super::usage::Usage;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The usage recorded in session `id` since `since_us` (microseconds).
pub(super) fn from_muse_session(id: &str, since_us: i64) -> Usage {
    let mut usage = Usage::default();
    let root = crate::config::home_dir().join(".local/share/muse/sessions");
    let Some(file) = find(&root, id, 3) else {
        return usage;
    };
    std::fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|m| m["recorded_at"].as_i64().unwrap_or(0) >= since_us)
        .for_each(|m| usage.take(&m));
    usage
}

/// The session.jsonl of session `id`, `depth` folders (year, month, day) below `dir`.
fn find(dir: &Path, id: &str, depth: u32) -> Option<PathBuf> {
    if depth == 0 {
        let file = dir.join(id).join("session.jsonl");
        return file.exists().then_some(file);
    }
    let mut subs: Vec<_> = std::fs::read_dir(dir).ok()?.flatten().collect();
    subs.sort_by_key(|e| std::cmp::Reverse(e.file_name())); // the newest days first
    subs.iter().find_map(|e| find(&e.path(), id, depth - 1))
}
