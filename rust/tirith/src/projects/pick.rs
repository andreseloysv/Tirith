//! The macOS folder picker.
use crate::app::{bad, Shared, R};
use crate::config::home_dir;
use axum::{extract::State, Json};
use serde_json::json;
use std::{path::Path, path::PathBuf, time::Duration};

/// One folder picker on screen at a time.
static PICKING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
/// AppKit's open panel through JXA, floating above every window: since macOS 14 a helper process may not come to the front, so AppleScript's `choose folder` opened behind the page. It prints the folder, or nothing when cancelled.
const PANEL: &str = include_str!("../../assets/panel.js");

/// The macOS folder picker, opened by the daemon in the owner's session (one at a time): the folder, or "" when cancelled.
pub(super) async fn pick(State(app): State<Shared>) -> R {
    crate::sandbox::needs_mac("the folder picker (type the path)").map_err(bad)?;
    let Ok(_one) = PICKING.try_lock() else {
        return Err(bad("the folder picker is already open".into()));
    };
    let out = tokio::process::Command::new("osascript")
        .args(["-l", "JavaScript", "-e", PANEL])
        .arg(start(&app))
        .kill_on_drop(true)
        .output();
    let out = tokio::time::timeout(Duration::from_secs(600), out)
        .await
        .map_err(|_| bad("the folder picker was open too long".into()))?
        .map_err(|e| bad(format!("no folder picker here ({e}): type the path")))?;
    out.status.success().then_some(()).ok_or_else(|| {
        bad(format!(
            "the folder picker did not open: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    })?;
    Ok(Json(
        json!({"folder": String::from_utf8_lossy(&out.stdout).trim()}),
    ))
}

/// Where the picker opens: next to the first project, else the home folder.
fn start(app: &Shared) -> PathBuf {
    app.cfg()
        .projects
        .iter()
        .find(|p| !p.repo.is_empty())
        .and_then(|p| p.repo().parent().map(Path::to_path_buf))
        .filter(|d| d.is_dir())
        .unwrap_or_else(home_dir)
}
