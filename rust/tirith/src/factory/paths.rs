//! Where the factory folder is, and the texts it holds.
use super::values::stem;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The factory folder: config.toml's `factory` key, else the repo's own when this build sits in a checkout.
pub(super) fn dir(overlay: &toml::Table) -> Option<PathBuf> {
    let here = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../factory"));
    overlay
        .get("factory")
        .and_then(|v| v.as_str())
        .map(crate::config::expand)
        .or_else(|| here.join("factory.toml").is_file().then_some(here))
}

/// The files of the folder's `prompts` and `evals`.
pub(super) fn text_files(dir: &Path) -> impl Iterator<Item = std::fs::DirEntry> + '_ {
    ["prompts", "evals"]
        .into_iter()
        .filter_map(|d| std::fs::read_dir(dir.join(d)).ok())
        .flatten()
        .flatten()
}

/// The factory's texts by file stem: the prompts and the eval rubrics alike.
pub(super) fn prompts(dir: &Path) -> BTreeMap<String, String> {
    text_files(dir)
        .filter(|e| e.path().extension().is_some_and(|x| x == "md"))
        .filter_map(|e| {
            Some((
                stem(&e.path().to_string_lossy()),
                std::fs::read_to_string(e.path()).ok()?,
            ))
        })
        .collect()
}
