//! An agent's hooks pointed at Tirith, in its own settings file: everything else there stays, and the first change keeps a copy.
use super::catalog::Known;
use super::hook_lines::{command, others, EVENTS};
use serde_json::{json, Value};
use std::path::Path;

/// Writes the hooks into `file`; whether it changed.
pub fn install(
    known: &Known,
    file: &Path,
    exe: &Path,
    home: Option<&Path>,
) -> Result<bool, String> {
    let failed = |e: std::io::Error| format!("{}: {e}", file.display());
    let text = match std::fs::read_to_string(file) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(failed(e)),
    };
    let was: Value = match text.as_deref().filter(|t| !t.trim().is_empty()) {
        Some(t) => serde_json::from_str(t)
            .map_err(|e| format!("{} is no JSON, so it stays as it is: {e}", file.display()))?,
        None => json!(known.new_settings),
    };
    let mut now = was.clone();
    let whole = now.is_object() && (now["hooks"].is_null() || now["hooks"].is_object());
    whole
        .then_some(())
        .ok_or_else(|| format!("{}: its hooks are not a table", file.display()))?;
    let flat = known.hooks == "copilot";
    if flat {
        now["version"] = json!(1);
    }
    for event in EVENTS {
        let line = command(exe, home, &known.alias, event);
        let mut list = others(&now["hooks"][event]);
        list.push(if flat {
            json!({"type": "command", "bash": line, "timeoutSec": 10})
        } else {
            json!({"hooks": [{"type": "command", "command": line, "timeout": 10}]})
        });
        now["hooks"][event] = json!(list);
    }
    if now == was && text.is_some() {
        return Ok(false);
    }
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(failed)?;
    }
    let mut kept = file.as_os_str().to_owned();
    kept.push(".before-tirith");
    if text.is_some() && !Path::new(&kept).exists() {
        std::fs::copy(file, &kept).map_err(failed)?;
    }
    let out = serde_json::to_string_pretty(&now).map_err(|e| e.to_string())?;
    std::fs::write(file, out + "\n").map_err(failed)?;
    Ok(true)
}
