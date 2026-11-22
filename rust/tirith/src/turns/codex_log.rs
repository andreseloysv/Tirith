//! Codex's own log of a thread, for the one thing its output leaves out: the model a turn ran on.
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// The model of the thread's last turn (its rollout's last `turn_context`).
pub(super) fn model(thread: &str) -> Option<String> {
    let home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::config::home_dir().join(".codex"));
    let file = find(&home.join("sessions"), &format!("{thread}.jsonl"), 3)?;
    BufReader::new(std::fs::File::open(file).ok()?)
        .lines()
        .map_while(Result::ok)
        .filter(|l| l.contains("\"turn_context\""))
        .filter_map(|l| serde_json::from_str::<Value>(&l).ok())
        .filter(|v| v["type"] == "turn_context")
        .filter_map(|v| v["payload"]["model"].as_str().map(String::from))
        .last()
}

/// The rollout whose name ends in `name`, `depth` folders (year, month, day) below `dir`.
fn find(dir: &Path, name: &str, depth: u32) -> Option<PathBuf> {
    let mut all: Vec<_> = std::fs::read_dir(dir).ok()?.flatten().collect();
    all.sort_by_key(|e| std::cmp::Reverse(e.file_name())); // the newest days first
    if depth == 0 {
        return all
            .iter()
            .find(|e| e.file_name().to_string_lossy().ends_with(name))
            .map(|e| e.path());
    }
    all.iter().find_map(|e| find(&e.path(), name, depth - 1))
}
