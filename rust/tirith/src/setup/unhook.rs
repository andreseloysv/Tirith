//! Uninstalling NeoBot takes its hooks out of every agent's settings (`neobot hooks remove`, run by the
//! uninstaller): left there, each tool call of an agent would run a program that is gone. Whose they were is
//! kept in NeoBot's home (`unhooked`), and the next NeoBot that starts there puts them back (`rehook.rs`):
//! an update is an uninstall and an install.
use super::{catalog, context::settings_of, hooks};
use crate::config::Config;
use std::path::Path;

/// The agents whose hooks an uninstall took out, one alias a line.
pub(super) const RECORD: &str = "unhooked";

pub(super) fn recorded(home: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(home.join(RECORD)).unwrap_or_default();
    let lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    lines.map(String::from).collect()
}

/// Writes down who waits for their hooks; nobody: the record goes.
pub(super) fn record(home: &Path, aliases: &[String]) -> std::io::Result<()> {
    if aliases.is_empty() {
        return match std::fs::remove_file(home.join(RECORD)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    std::fs::create_dir_all(home)?;
    std::fs::write(home.join(RECORD), aliases.join("\n") + "\n")
}

/// Takes NeoBot's hooks out of every known agent's settings file; the agents that wait to get them back.
pub fn remove(home: &Path) -> Result<Vec<String>, String> {
    let cfg = Config::load_at(home);
    let (mut gone, mut failed) = (recorded(home), vec![]);
    for known in catalog::all() {
        match settings_of(&cfg, &known).map(|file| hooks::remove(&file)) {
            Some(Ok(true)) if !gone.contains(&known.alias) => gone.push(known.alias),
            Some(Err(why)) => failed.push(why),
            _ => {}
        }
    }
    record(home, &gone).map_err(|e| format!("{}: {e}", home.display()))?;
    failed
        .is_empty()
        .then_some(gone)
        .ok_or_else(|| failed.join("\n"))
}

/// `neobot hooks remove`: what the uninstaller runs. The owner's alone: the hooks are how the rules hear an agent.
pub fn cli() -> Result<(), String> {
    if super::from_agent() {
        return Err("neobot hooks remove is the owner's: run it in your own terminal".into());
    }
    let gone = remove(&crate::config::tirith_home())?;
    match gone.is_empty() {
        true => println!("no agent had NeoBot's hooks"),
        false => println!(
            "NeoBot's hooks are out of: {} (they come back when NeoBot next starts)",
            gone.join(", ")
        ),
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../checks/unhook.rs"]
mod tests;
