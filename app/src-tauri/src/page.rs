//! The page as the window opens it.

use std::path::Path;
use std::process::{Command, Stdio};

/// The page signed in as the owner: a link from `neobot open link` that works once. The bare page, which says what is missing, without one.
pub fn link(port: u16, tirith: &Path) -> String {
    let out = crate::daemon::quiet(Command::new(tirith).args(["open", "link"]).stdin(Stdio::null())).output();
    let link = out.ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    let base = format!("http://127.0.0.1:{port}/");
    link.filter(|l| l.starts_with(&base)).unwrap_or(base)
}
