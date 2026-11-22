//! The git blob hashes of files as they are now.
use serde_json::{Map, Value};
use std::path::PathBuf;

/// `name -> git hash-object` of each file that exists (one git process for all of them).
pub(super) fn hashes(files: Vec<(String, PathBuf)>) -> Map<String, Value> {
    let files: Vec<(String, PathBuf)> = files.into_iter().filter(|(_, p)| p.is_file()).collect();
    let mut out = Map::new();
    if files.is_empty() {
        return out;
    }
    let run = std::process::Command::new("git")
        .arg("hash-object")
        .args(files.iter().map(|(_, p)| p))
        .output();
    let Some(done) = run.ok().filter(|o| o.status.success()) else {
        return out;
    };
    let text = String::from_utf8_lossy(&done.stdout);
    for ((name, _), id) in files.into_iter().zip(text.lines()) {
        out.insert(name, Value::String(id.trim().into()));
    }
    out
}
