//! The models Codex lists in its local cache, with the reasoning efforts each one supports.
use serde_json::{json, Value};

pub fn catalog() -> Vec<Value> {
    let home = std::env::var_os("CODEX_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| crate::config::home_dir().join(".codex"));
    let data: Value = std::fs::read(home.join("models_cache.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    data["models"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|m| m["visibility"] == "list")
        .filter_map(|m| m["slug"].as_str().map(|slug| json!({"slug": slug, "name": m["display_name"], "efforts": m["supported_reasoning_levels"].as_array().into_iter().flatten().filter_map(|e| e["effort"].as_str()).collect::<Vec<_>>()})))
        .collect()
}
