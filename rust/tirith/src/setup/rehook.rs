//! The hooks an uninstall took out (`unhook.rs`) come back when NeoBot starts again in the same home, pointed
//! at the binary that runs now: the other half of an update.
use super::unhook::{record, recorded, RECORD};
use super::{catalog, context::settings_of, find, hooks, trust};
use crate::config::{home_dir, Config};
use std::path::Path;

/// One agent's hooks back, at `exe`; Codex is asked to trust them again when they changed.
fn back(cfg: &Config, home: &Path, exe: &Path, alias: &str) -> Result<(), String> {
    let known = catalog::known(alias).ok_or("not an agent of this NeoBot")?;
    let file = settings_of(cfg, &known).ok_or("no settings file")?;
    let named = (home != home_dir().join(".tirith")).then_some(home);
    hooks::install(&known, &file, exe, named)?;
    let program = cfg
        .factory
        .command(alias)
        .as_deref()
        .and_then(find::program);
    match program.filter(|_| known.hooks == "codex") {
        Some(program) => trust::trust(&program).map(drop),
        None => Ok(()),
    }
}

/// Puts back the hooks an uninstall took out, pointed at `exe`; the agents that have them again. One that
/// could not get them stays written down, for the next start.
pub fn restore(home: &Path, exe: &Path) -> Vec<String> {
    let aliases = recorded(home);
    if aliases.is_empty() {
        return aliases;
    }
    let cfg = Config::load_at(home);
    let (done, left): (Vec<String>, Vec<String>) = aliases
        .into_iter()
        .partition(|alias| back(&cfg, home, exe, alias).is_ok());
    let _ = record(home, &left);
    done
}

/// At the daemon's start, in the background (Codex may have to trust them again).
pub fn at_start(home: &Path) {
    let exe = crate::app::exe().filter(|_| home.join(RECORD).is_file());
    if let Some((exe, home)) = exe.map(|exe| (exe, home.to_path_buf())) {
        std::thread::spawn(move || {
            println!("neobot: hooks back for {}", restore(&home, &exe).join(", "))
        });
    }
}
