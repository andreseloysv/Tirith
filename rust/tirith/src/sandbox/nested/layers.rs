//! A setting as Codex reads it: `-c`, then the chosen profile, the trusted project's file, and its config.toml.
use super::asked::Asked;
use super::values::{deeper, file, value};
use std::path::{Path, PathBuf};
use toml::{Table, Value};

pub(super) struct Layers {
    pub(super) sets: Vec<(String, Value)>,
    pub(super) files: Vec<Table>,
    /// The folder's project is one Codex was told about.
    pub known_project: bool,
}

impl Layers {
    pub(super) fn of(a: &Asked, home: &Path, dir: &Path) -> Layers {
        let base = if a.ignore_config {
            Table::new()
        } else {
            file(&home.join("config.toml"))
        };
        let repo = dir
            .ancestors()
            .find(|d| d.join(".git").exists())
            .unwrap_or(dir);
        let trust = |at: &Path| -> Option<String> {
            let of = |at: &Path| base.get("projects")?.get(at.to_string_lossy().as_ref());
            let told = of(at).or_else(|| of(&at.canonicalize().ok()?))?;
            Some(told.get("trust_level")?.as_str()?.to_string())
        };
        let (told, mut files) = (trust(dir).or_else(|| trust(repo)), vec![]);
        if let Some(name) = super::profiles::chosen(a, &base).filter(|n| !n.contains('/')) {
            files.push(file(
                &home.join(PathBuf::from(format!("{name}.config.toml"))),
            ));
            let old = base
                .get("profiles")
                .and_then(|p| p.get(&name))
                .and_then(|p| p.as_table());
            files.extend(old.cloned());
        }
        if told.as_deref() == Some("trusted") {
            files.push(file(&repo.join(".codex/config.toml")));
        }
        files.push(base.clone());
        let sets = a
            .sets
            .iter()
            .rev()
            .map(|(k, v)| (k.clone(), value(v)))
            .collect();
        Layers {
            sets,
            files,
            known_project: told.is_some(),
        }
    }

    /// The setting at `key` (`a.b`).
    pub(super) fn get(&self, key: &str) -> Option<Value> {
        for (set, v) in &self.sets {
            if set == key {
                return Some(v.clone());
            }
            if let Some(rest) = key.strip_prefix(&format!("{set}.")) {
                if let Some(found) = deeper(v, rest) {
                    return Some(found.clone());
                }
            }
        }
        self.files.iter().find_map(|t| {
            let (first, rest) = key.split_once('.').map_or((key, ""), |(a, b)| (a, b));
            let top = t.get(first)?;
            if rest.is_empty() {
                Some(top.clone())
            } else {
                deeper(top, rest).cloned()
            }
        })
    }

    pub(super) fn is(&self, key: &str) -> bool {
        self.get(key).and_then(|v| v.as_bool()) == Some(true)
    }
}
