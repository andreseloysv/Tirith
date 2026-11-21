//! Keeping the bots: one change at a time, written into config.toml and reloaded.
use super::list::all;
use super::written::written;
use crate::app::{App, Shared};
use serde_json::{json, Value};
use std::sync::Mutex;
static ONE: Mutex<()> = Mutex::new(());
/// Writes the bots into config.toml and reloads it.
pub(super) fn store(app: &App, bots: &[Value]) -> Result<(), String> {
    crate::projects::rewrite(app, |text| written(text, bots))
}
/// Reads the bots, changes them with `f` and keeps the result, one change at a time; an Err keeps them as they were.
pub(super) fn with<T>(
    app: &Shared,
    f: impl FnOnce(&mut Vec<Value>) -> Result<T, String>,
) -> Result<T, String> {
    let _one = ONE.lock().unwrap();
    let mut bots = all(app);
    let out = f(&mut bots)?;
    store(app, &bots)?;
    app.emit("bots", json!({}));
    Ok(out)
}
pub(super) fn find<'a>(bots: &'a mut [Value], id: &str) -> Result<&'a mut Value, String> {
    bots.iter_mut()
        .find(|b| b["id"] == id)
        .ok_or(format!("no bot '{id}'"))
}
