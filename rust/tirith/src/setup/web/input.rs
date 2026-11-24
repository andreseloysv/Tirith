//! The input of the program a setup runs, held open while it may ask for a code the owner pastes on the page.
use super::job;
use std::collections::HashMap;
use std::io::Write;
use std::process::ChildStdin;
use std::sync::{LazyLock, Mutex};

static INPUT: LazyLock<Mutex<HashMap<String, ChildStdin>>> = LazyLock::new(Default::default);

fn all() -> std::sync::MutexGuard<'static, HashMap<String, ChildStdin>> {
    INPUT.lock().unwrap_or_else(|e| e.into_inner())
}

/// Held until its setup ends.
pub fn hold(alias: &str, input: ChildStdin) {
    all().insert(alias.into(), input);
}

pub fn release(alias: &str) {
    all().remove(alias);
}

/// A code the owner pasted, handed to the program that asked for it.
pub fn give(alias: &str, text: &str) -> Result<(), String> {
    let mut all = all();
    let input = all.get_mut(alias).ok_or("nothing waits for a code now")?;
    writeln!(input, "{}", text.trim())
        .and_then(|_| input.flush())
        .map_err(|e| e.to_string())?;
    drop(all);
    job::with(alias, |job| job.paste = false);
    Ok(())
}
