//! The collector: merge inputs with live facts into `state.json` (`collector.py`).
//!
//! Ported so far: `projects[]` with `id/name/color/folder/auto/roadmap/plans`. Every other section of the
//! Python state is missing from the Rust state on purpose; the parity harness reports them as
//! "not ported yet" (see `docs/RUST-PORT.md`).

use crate::home::{load_config, Config};
use crate::projects::{discover, Entry};
use crate::roadmap;
use crate::state::{ProjectRow, State, STATE_VERSION};
use crate::store::write_json_atomic;
use crate::timeutil::iso;
use chrono::{DateTime, Local, Utc};
use serde_json::Value;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

/// Owns one home directory and builds `state.json` cycle by cycle.
pub struct Collector {
    pub home: PathBuf,
}

impl Collector {
    pub fn new(home: &Path) -> Self {
        Collector { home: home.to_path_buf() }
    }

    /// Build (but do not write) one state payload.
    pub fn build_state(&self, now: DateTime<Utc>) -> State {
        let config = load_config(&self.home);
        // Folders of lanes, events and rooms feed discovery in Python; those inputs are not ported yet.
        let entries = discover(&self.home, &config, &[]);
        let agents: Vec<Value> = Vec::new(); // agent rows come with the agents lane
        let today = now.with_timezone(&Local).date_naive();
        State {
            version: STATE_VERSION,
            generated_at: iso(now),
            projects: project_rows(&config, &entries, &agents, today),
        }
    }

    /// One collector cycle: build the state and write it atomically.
    pub fn cycle(&self, now: DateTime<Utc>) -> io::Result<State> {
        let state = self.build_state(now);
        let payload = serde_json::to_value(&state).map_err(io::Error::other)?;
        write_json_atomic(&self.home.join("state.json"), &payload, true)?;
        Ok(state)
    }

    /// A single cycle (`tirith collect --once`).
    pub fn run_once(&self) -> io::Result<State> {
        self.cycle(Utc::now())
    }
}

/// Registered projects in config order, then the auto-discovered ones.
fn project_rows(config: &Config, entries: &[Entry], agents: &[Value], today: chrono::NaiveDate) -> Vec<ProjectRow> {
    let folder_of = |id: &str| entries.iter().find(|e| e.id == id).and_then(|e| e.folder.clone());
    let row = |id: &str, name: &str, color: &str, folder: Option<PathBuf>, auto: bool| ProjectRow {
        id: id.to_string(),
        name: name.to_string(),
        color: color.to_string(),
        roadmap: roadmap::build(folder.as_deref(), id, agents, today),
        plans: roadmap::build_plans(folder.as_deref(), id, agents),
        folder: folder.map(|f| f.to_string_lossy().into_owned()),
        auto,
    };
    let mut known: HashSet<&str> = HashSet::new();
    let mut rows = Vec::new();
    for proj in &config.projects {
        known.insert(&proj.id);
        rows.push(row(&proj.id, &proj.name, &proj.color, folder_of(&proj.id), false));
    }
    for entry in entries {
        if known.insert(&entry.id) {
            rows.push(row(&entry.id, &entry.name, &entry.color, entry.folder.clone(), true));
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::read_json;
    use std::fs;

    #[test]
    fn once_writes_projects_with_their_roadmap() {
        let home = tempfile::tempdir().unwrap();
        let repo = tempfile::tempdir().unwrap();
        fs::create_dir_all(repo.path().join("docs")).unwrap();
        fs::write(repo.path().join("docs/TODO.md"), "## 1. Only step\n- [x] done\n- [ ] open\n").unwrap();
        fs::write(
            home.path().join("config.toml"),
            format!("[[projects]]\nid = \"demo\"\nname = \"Demo\"\nrepo = {:?}\n[[projects]]\nid = \"bare\"\n", repo.path()),
        )
        .unwrap();

        let state = Collector::new(home.path()).run_once().unwrap();
        assert_eq!(state.projects.len(), 2);
        let demo = &state.projects[0];
        assert_eq!((demo.id.as_str(), demo.color.as_str(), demo.auto), ("demo", "#888888", false));
        let road = demo.roadmap.as_ref().unwrap();
        assert_eq!((road.done, road.total, road.steps[0].status.as_str()), (1, 2, "ready"));
        assert_eq!(demo.plans.len(), 1);
        assert!(state.projects[1].roadmap.is_none() && state.projects[1].folder.is_none());

        let written = read_json(&home.path().join("state.json")).unwrap();
        assert_eq!(written["version"], 1);
        assert_eq!(written["projects"][0]["roadmap"]["steps"][0]["items"][0]["who"], "agent");
        assert!(written["projects"][1]["roadmap"].is_null());
    }
}
