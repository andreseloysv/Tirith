//! Where and when Codex runs, as its own environment note would say it: that note goes, for it calls the sandbox none.
use std::path::Path;

/// «Working directory: …. Shell: zsh. Date: 2026-11-22 (Europe/Berlin).»
pub(super) fn here(dir: &Path) -> String {
    let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let shell = std::env::var("SHELL").unwrap_or_default();
    let shell = shell
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("zsh");
    let date = chrono::Local::now().format("%Y-%m-%d");
    format!(
        "Working directory: {}.\nShell: {shell}. Date: {date} ({}).",
        dir.display(),
        zone()
    )
}

/// The time zone's name, as the system links it; else its offset.
fn zone() -> String {
    let linked = std::fs::read_link("/etc/localtime").ok();
    let name = linked.and_then(|p| {
        let path = p.display().to_string();
        path.split_once("zoneinfo/")
            .map(|(_, name)| name.to_string())
    });
    name.unwrap_or_else(|| chrono::Local::now().format("UTC%:z").to_string())
}
