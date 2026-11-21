//! At start: every bot's folder, and the bots that used to live in kv moved to config.toml once.
use super::folder::{folder, SKILLS};
use super::list::all;
use super::store::store;
use crate::app::Shared;
use crate::server::s;
use serde_json::Value;
/// The old key's name lives here alone, so nothing reads it as storage any more.
const OLD: &str = "bots";
/// Every bot (the system ones too): its folder under manager/, its memory.md and its skills folder, from the start.
fn folders(app: &Shared) {
    all(app)
        .iter()
        .map(|b| folder(app, &s(b, "key")))
        .for_each(|d| {
            drop((
                std::fs::create_dir_all(d.join(SKILLS)),
                std::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(false)
                    .open(d.join("memory.md")),
            ))
        });
}
/// The old bots' routines' last runs move to their own kv stamps.
fn stamps(app: &Shared, old: &[Value]) {
    old.iter().for_each(|b| {
        b["routines"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| !s(r, "last").is_empty())
            .for_each(|r| {
                app.set_kv(
                    &format!("routine:{}:{}", s(b, "id"), s(r, "id")),
                    Some(&s(r, "last")),
                )
            })
    });
}
/// At start, the bots that used to live in kv move to config.toml once: one that is already a role there wins, and their routines' last runs move to their own kv stamps; only then does the old entry go.
pub fn start(app: Shared) {
    folders(&app);
    let old: Vec<Value> = app
        .kv(OLD)
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default();
    let Some(old) = (!old.is_empty()).then_some(old) else {
        return;
    };
    let mut bots = all(&app);
    let have: Vec<String> = bots.iter().map(|b| s(b, "id")).collect();
    bots.extend(old.iter().filter(|o| !have.contains(&s(o, "id"))).cloned());
    if store(&app, &bots).is_ok() {
        stamps(&app, &old);
        app.set_kv(OLD, None);
    }
}
