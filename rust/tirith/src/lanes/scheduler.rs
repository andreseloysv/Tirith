//! The scheduler loop: every few seconds (or when woken) it keeps the house, watches the running lanes and starts the next one when there is room.
use super::builds::free_idle_builds;
use super::disk::{free_gb, warn_low_disk};
use super::pick::start_next;
use super::sweep::sweep_cleanups;
use super::watch::watch_lane;
use crate::app::{Lane, Shared};
use crate::{factory::Factory, manager};
use std::collections::HashMap;
use std::time::Duration;

/// What the scheduler remembers between passes.
#[derive(Default)]
struct Memory {
    rr: usize,
    said: HashMap<String, String>,
    said_error: Option<String>,
    tick: u64,
    seen: HashMap<String, (String, u64)>,
}
/// config.toml changed by hand: the page gets the projects again, and a new configuration error is said once.
fn reload_config(app: &Shared, said_error: &mut Option<String>) {
    let _ = app
        .reload_config(false)
        .then(|| app.emit("projects", serde_json::json!(crate::projects::list(app))));
    let cfg = app.cfg();
    if cfg.error != *said_error {
        said_error.clone_from(&cfg.error);
        said_error.iter().for_each(|e| {
            let text = format!("Configuration: {e}");
            manager::emit_event(app, "", "factory.invalid", &text, None)
        });
    }
}
/// The one-minute load average per core.
fn load() -> f64 {
    crate::os::load()
}
/// Disk diet when space is low, then on its own ticks the low-disk card, the sweep, stale questions and the bots.
async fn housekeeping(app: &Shared, lanes: &[Lane], f: &Factory, tick: &mut u64) {
    if free_gb(&app.home.join("lanes")) < f.limits.low_disk_gb {
        free_idle_builds(app, lanes).await; // first, so neither the owner's card nor the pause comes when this is enough
    }
    if tick.is_multiple_of(100) {
        warn_low_disk(app).await;
        sweep_cleanups(app, lanes).await; // every ~5 minutes; the first pass runs at startup, applying the rules to everything that already meets them
    }
    let _ = tick
        .is_multiple_of(10)
        .then(|| (manager::close_stale_questions(app), crate::bots::tick(app))); // every ~30 s
    *tick += 1;
}
/// Room for another lane: under `max_parallel`, and under `max_load` unless nothing runs.
fn has_room(f: &Factory, running: usize, load: f64) -> bool {
    !f.limits.max_parallel.is_some_and(|m| running >= m)
        && !(running > 0 && load >= f.limits.max_load)
}
/// Every few seconds (or when woken): stop idle lanes, pick up ended processes, and when the machine has room start the oldest queued lane (manual, sent back or answered) or else the autopilot's next item.
pub async fn scheduler(app: Shared) {
    let mut m = Memory::default();
    loop {
        let _ = tokio::time::timeout(Duration::from_secs(3), app.sched_wake.notified()).await;
        reload_config(&app, &mut m.said_error);
        let cfg = app.cfg();
        let f = &cfg.factory;
        let load = load();
        let lanes: Vec<Lane> = app.lanes.lock().unwrap().values().cloned().collect();
        let running = lanes.iter().filter(|l| l.pid.is_some()).count();
        housekeeping(&app, &lanes, f, &mut m.tick).await;
        app.emit("load", serde_json::json!({"load": (load * 100.0).round() / 100.0, "running": running, "max": f.limits.max_parallel}));
        for l in lanes.iter().filter(|l| l.pid.is_some()) {
            watch_lane(&app, l, &mut m.seen, f).await;
        }
        if has_room(f, running, load) {
            start_next(&app, &cfg, &lanes, &mut m.rr, &mut m.said).await;
        }
    }
}
