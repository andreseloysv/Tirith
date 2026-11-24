//! The PATH agents and checks run with.
use super::App;
use std::path::{Path, PathBuf};

/// Where this binary is on disk, also after a newer one took its place (Linux then adds " (deleted)" to the path).
pub fn exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?.display().to_string();
    Some(named(PathBuf::from(
        exe.strip_suffix(" (deleted)").unwrap_or(&exe),
    )))
}

/// Started as its alias `tirith`, the binary still is `neobot`: what it writes into hooks and services names the new command.
fn named(exe: PathBuf) -> PathBuf {
    if exe.file_stem().is_none_or(|s| s != "tirith") {
        return exe;
    }
    let real = std::fs::canonicalize(&exe).ok();
    let beside = exe.with_file_name(if cfg!(windows) {
        "neobot.exe"
    } else {
        "neobot"
    });
    let found = real.filter(|r| r.file_stem().is_some_and(|s| s == "neobot"));
    found
        .or_else(|| beside.is_file().then_some(beside))
        .unwrap_or(exe)
}

impl App {
    /// PATH for agents and checks: this binary's folder first, so `neobot` is this daemon's client, and `<home>/bin` right after it, so `tirith` is too, and `~/.cargo/bin`, `/opt/homebrew/bin`, `/usr/local/bin` last when they exist.
    pub fn child_path(&self) -> String {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(Path::to_path_buf));
        let own = self.home.join("bin");
        super::alias::link_names(&own);
        path_with_cargo(
            exe_dir.as_deref(),
            &format!(
                "{}:{}",
                own.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
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
