//! Per-project `tirith.toml` (`manifest.py`). Only the fields the roadmap needs are ported so far:
//! `roadmap`, `deadline` and `plans`. Pipelines, hooks, rules and `problems` belong to the manifest lane.

use crate::pytext::{get_str, str_of, strip};
use std::fs;
use std::path::{Path, PathBuf};
use toml::{Table, Value};

pub const FILENAME: &str = "tirith.toml";

#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    pub path: PathBuf,
    pub exists: bool,
    pub roadmap: Option<String>,
    pub deadline: Option<String>,
    pub plans: Vec<String>,
}

fn blank_to_none(text: Option<String>) -> Option<String> {
    text.map(|t| strip(&t).to_string()).filter(|t| !t.is_empty())
}

/// Read `<folder>/tirith.toml`. A missing, unreadable or broken file gives no roadmap fields.
pub fn load(folder: &Path) -> Manifest {
    let path = folder.join(FILENAME);
    let mut manifest = Manifest { path, exists: false, roadmap: None, deadline: None, plans: Vec::new() };
    match fs::read_to_string(&manifest.path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return manifest,
        Err(_) => {
            manifest.exists = true;
            return manifest;
        }
        Ok(text) => {
            manifest.exists = true;
            if let Ok(data) = text.parse::<Table>() {
                manifest.roadmap = blank_to_none(get_str(&data, "roadmap"));
                manifest.deadline = blank_to_none(get_str(&data, "deadline"));
                if let Some(Value::Array(plans)) = data.get("plans") {
                    manifest.plans = plans.iter().map(|p| strip(&str_of(p)).to_string()).filter(|p| !p.is_empty()).collect();
                }
            }
        }
    }
    manifest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_roadmap_fields() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!load(dir.path()).exists);
        fs::write(
            dir.path().join(FILENAME),
            "roadmap = \" plan/ROAD.md \"\ndeadline = \"2026-11-04T21:00:00-05:00\"\nplans = [\"a.md\", \" \", \"b.md\"]\n",
        )
        .unwrap();
        let m = load(dir.path());
        assert_eq!(m.roadmap.as_deref(), Some("plan/ROAD.md"));
        assert_eq!(m.deadline.as_deref(), Some("2026-11-04T21:00:00-05:00"));
        assert_eq!(m.plans, ["a.md", "b.md"]);
    }

    #[test]
    fn broken_toml_gives_nothing() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(FILENAME), "roadmap = [").unwrap();
        let m = load(dir.path());
        assert!(m.exists && m.roadmap.is_none() && m.plans.is_empty());
    }
}
