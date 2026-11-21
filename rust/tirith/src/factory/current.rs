//! The factory version the daemon runs now, reloaded under config.toml's overlay; a bad version keeps the last good one.
use super::embedded::{embedded_prompts, TEXT};
use super::loaded::Factory;
use super::parse::parse;
use super::paths::{dir, prompts};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

static CURRENT: RwLock<Option<Arc<Factory>>> = RwLock::new(None);
static ERROR: Mutex<Option<String>> = Mutex::new(None);

/// The last good factory, or the embedded one before any loaded.
pub fn current() -> Arc<Factory> {
    static EMBEDDED: OnceLock<Arc<Factory>> = OnceLock::new();
    CURRENT.read().unwrap().clone().unwrap_or_else(|| {
        EMBEDDED
            .get_or_init(|| Arc::new(Factory::default()))
            .clone()
    })
}

/// Why the newest factory version did not load; the last good one keeps running.
pub fn error() -> Option<String> {
    ERROR.lock().unwrap().clone()
}

/// The factory folder's version under the overlay, or the embedded one when no folder is found.
fn attempt(overlay: &toml::Table) -> Result<Factory, String> {
    dir(overlay).map_or_else(
        || parse(TEXT, embedded_prompts(), overlay),
        |d| {
            std::fs::read_to_string(d.join("factory.toml"))
                .map_err(|e| format!("factory: {}: {e}", d.display()))
                .and_then(|text| parse(&text, prompts(&d), overlay))
        },
    )
}

/// (Re)loads the factory under config.toml's overlay; a bad version keeps the last good one and is remembered in `error()`.
pub fn load(overlay: &toml::Table) -> Arc<Factory> {
    let f = match attempt(overlay) {
        Ok(f) => Arc::new(f),
        Err(e) => {
            *ERROR.lock().unwrap() = Some(e);
            return current();
        }
    };
    *ERROR.lock().unwrap() = None;
    *CURRENT.write().unwrap() = Some(f.clone());
    f
}
