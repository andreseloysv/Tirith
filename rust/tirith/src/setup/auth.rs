//! What signs one agent in without a login (a token) or says where it lives (an enterprise's host): kept in `<home>/auth/<alias>.env` (0600) and handed to that agent's processes alone.
use crate::app::Shared;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

fn file(home: &Path, alias: &str) -> PathBuf {
    home.join("auth").join(format!("{alias}.env"))
}

/// The agent's variables.
pub fn env(home: &Path, alias: &str) -> Vec<(String, String)> {
    std::fs::read_to_string(file(home, alias))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

/// Keeps `value` as the agent's `var`, readable by the owner alone.
pub fn keep(home: &Path, alias: &str, var: &str, value: &str) -> Result<(), String> {
    let plain = !value.is_empty() && value.bytes().all(|b| b.is_ascii_graphic());
    plain
        .then_some(())
        .ok_or("that is no value for it: it is one word, with no spaces")?;
    let mut vars = env(home, alias);
    vars.retain(|(k, _)| k != var);
    vars.push((var.to_string(), value.to_string()));
    let text: String = vars.iter().map(|(k, v)| format!("{k}={v}\n")).collect();
    let path = file(home, alias);
    let failed = |e: std::io::Error| format!("{}: {e}", path.display());
    let dir = path.parent().ok_or("no folder for the token")?;
    std::fs::create_dir_all(dir).map_err(failed)?;
    std::fs::set_permissions(dir, PermissionsExt::from_mode(0o700)).map_err(failed)?;
    let mut open = std::fs::OpenOptions::new();
    let mut out = open
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)
        .map_err(failed)?;
    out.set_permissions(PermissionsExt::from_mode(0o600))
        .and_then(|_| out.write_all(text.as_bytes()))
        .map_err(failed)
}

/// The agent's `var` goes, the rest stay; whether it was there.
pub fn forget(home: &Path, alias: &str, var: &str) -> Result<bool, String> {
    let vars = env(home, alias);
    let left: Vec<&(String, String)> = vars.iter().filter(|(k, _)| k != var).collect();
    if left.len() == vars.len() {
        return Ok(false);
    }
    let text: String = left.iter().map(|(k, v)| format!("{k}={v}\n")).collect();
    let path = file(home, alias);
    let failed = |e: std::io::Error| format!("{}: {e}", path.display());
    std::fs::write(&path, text).map_err(failed)?; // (it stays 0600)
    Ok(true)
}

/// The variables of the agent whose command `program` is; none for any other program.
pub fn for_program(app: &Shared, program: &str) -> Vec<(String, String)> {
    let base = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();
    let agents = app.cfg().factory.commands();
    let same = agents.iter().find(|(_, c)| c == program);
    let named = || agents.iter().find(|(_, c)| base(c) == base(program));
    same.or_else(named)
        .map(|(alias, _)| env(&app.home, alias))
        .unwrap_or_default()
}
