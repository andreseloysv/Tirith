//! The browser MCP's launcher: the checkout's, else the one the binary carries, written into NeoBot's home.
use std::path::{Path, PathBuf};

const PATH: &str = "tools/browser/launch.mjs";

/// The launcher to start, or none: a carried one only where the browser MCP itself is installed (`<home>/browser-mcp`).
pub fn launcher(home: &Path) -> Option<PathBuf> {
    if let Some(file) = super::checkout(PATH) {
        return Some(file);
    }
    let dir = home.join("browser-mcp");
    dir.join("node_modules/@playwright/mcp")
        .is_dir()
        .then_some(())?;
    let (file, body) = (dir.join("launch.mjs"), super::file(PATH)?);
    if std::fs::read(&file).ok().as_deref() != Some(body) {
        std::fs::write(&file, body).ok()?;
    }
    Some(file)
}
