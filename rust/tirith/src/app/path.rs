//! The PATH agents and checks run with.
use super::App;
use std::path::{Path, PathBuf};

impl App {
    /// PATH for agents and checks: this binary's folder first, so `tirith` is this daemon's client, and `~/.cargo/bin`, `/opt/homebrew/bin`, `/usr/local/bin` last when they exist.
    pub fn child_path(&self) -> String {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(Path::to_path_buf));
        path_with_cargo(
            exe_dir.as_deref(),
            &std::env::var("PATH").unwrap_or_default(),
            &crate::config::home_dir(),
            &["/opt/homebrew/bin", "/usr/local/bin"],
        )
    }
}

pub(super) fn path_with_cargo(
    exe_dir: Option<&Path>,
    rest: &str,
    home: &Path,
    tool_dirs: &[&str],
) -> String {
    let cargo = home.join(".cargo/bin");
    let mut parts: Vec<String> = exe_dir
        .iter()
        .map(|d| d.display().to_string())
        .chain(rest.split(':').filter(|p| !p.is_empty()).map(String::from))
        .collect();
    for dir in [cargo, home.join(".local/bin")]
        .into_iter()
        .chain(tool_dirs.iter().map(PathBuf::from))
    {
        let _ = (dir.is_dir() && !parts.iter().any(|p| Path::new(p) == dir))
            .then(|| parts.push(dir.display().to_string()));
    }
    parts.join(":")
}
