//! Where an agent's program is on this machine.
use super::catalog::Known;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Where installers put programs, besides PATH.
fn usual() -> Vec<PathBuf> {
    let home = crate::config::home_dir();
    let mut all: Vec<PathBuf> = [".local/bin", ".npm-global/bin", ".bun/bin", "bin"]
        .iter()
        .map(|d| home.join(d))
        .collect();
    all.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from));
    all
}

fn runs(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

/// The program called `name` (a path, or a name looked up), as an absolute path.
pub fn program(name: &str) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }
    if name.contains('/') {
        let path = crate::config::expand(name);
        return runs(&path).then(|| std::path::absolute(&path).unwrap_or(path));
    }
    let path = std::env::var("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .filter(|d| d.is_absolute())
        .chain(usual())
        .map(|d| d.join(name))
        .find(|p| runs(p))
}

/// An agent's program: config.toml's command when it runs, else the catalog's name.
pub fn agent(known: &Known, configured: Option<&str>) -> Option<PathBuf> {
    configured.and_then(program).or_else(|| program(&known.bin))
}
