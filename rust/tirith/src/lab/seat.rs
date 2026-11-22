//! Which folder a bot's turn runs in during a live experiment: a run of the routine is A (the bot's own folder, its main) or B (the branch's worktree, which is the bot's folder for that turn), alternating. A live session of the bot ends before a B run (at once, or at the end of the turn it runs) and after it, so each run's session starts in its own folder.
use super::store::{running, tree};
use crate::app::{App, Shared};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The run of an experiment on its way: from the routine's firing to the end of the turn it starts.
#[derive(Clone)]
pub struct Seat {
    pub experiment: i64,
    pub routine: String,
    pub variant: &'static str,
    pub tree: PathBuf,
    /// A shadow run's pair (its B runs beside it, `shadow.rs`); 0 in live mode.
    pub pair: i64,
}

/// The seats on their way, under the bot's own folder (a path, so two Tirith homes never share one).
pub(super) static SEATS: Mutex<BTreeMap<PathBuf, Seat>> = Mutex::new(BTreeMap::new());

/// A bot's own folder, its main's checkout: `<home>/manager/<key>`.
pub fn live(app: &App, key: &str) -> PathBuf {
    app.home.join("manager").join(key)
}

/// The folder a session or a read of `dir` (a bot's own folder or any other) uses now: inside a replay's turn its worktree; the branch's worktree while a live B run of that bot is on its way; else `dir` itself.
pub fn seat(dir: PathBuf) -> PathBuf {
    if let Some(replay) = super::replay::dir() {
        return replay; // inside a replay's turn, its worktree is every folder of the bot
    }
    if let Some(shadow) = super::shadow_open::dir() {
        return shadow; // inside a shadow B run, the branch's worktree is every folder of the bot
    }
    let seats = SEATS.lock().unwrap();
    let b = seats.get(&dir).filter(|s| s.variant == "B");
    b.map_or(dir, |s| s.tree.clone())
}

/// True while a run of the bot whose own folder is `dir` is on its way in an experiment.
pub fn seated(dir: &Path) -> bool {
    SEATS.lock().unwrap().contains_key(dir)
}

/// Routine `routine` of the bot `key` fires with `task`: with a live experiment running on it, this run is the variant after the last one (A first, then B, then A…), and a B run moves the bot into the branch's worktree until its turn ends; with a shadow one, this run is A and its B starts beside it.
pub fn fired(app: &Shared, key: &str, routine: &str, task: &str) {
    let Some(e) = running(app, key, routine).filter(|e| e.mode != "replay") else {
        return;
    };
    if e.mode == "shadow" {
        return super::shadow::fired(app, e, task);
    }
    let last = format!("lab:last:{}", e.id);
    let variant = match app.kv(&last).as_deref() {
        Some("A") => "B",
        _ => "A",
    };
    app.set_kv(&last, Some(variant));
    let s = Seat {
        experiment: e.id,
        routine: routine.into(),
        variant,
        tree: tree(app, &e),
        pair: 0,
    };
    if variant == "B" {
        super::workfiles::share(&live(app, key), &s.tree);
    }
    sit(app, key, s);
    super::fresh::session(app, key); // A and B alike
}

/// The run of the bot `key` on its way is `s`.
pub(super) fn sit(app: &App, key: &str, s: Seat) {
    SEATS.lock().unwrap().insert(live(app, key), s);
}

/// The turn of `key` that ran routine `routine` ended: the seat of its run, taken (the bot is back in its own folder).
pub fn taken(app: &App, key: &str, routine: &str) -> Option<Seat> {
    let mut seats = SEATS.lock().unwrap();
    let dir = live(app, key);
    let mine = seats
        .get(&dir)
        .is_some_and(|s| !routine.is_empty() && s.routine == routine);
    mine.then(|| seats.remove(&dir)).flatten()
}

/// The run of the bot `key` on its way ended without its turn's row (it never started, or its session ended): its seat is taken back, and its session ends, so the next starts in the bot's own folder with the recent chat.
pub fn released(app: &Shared, key: &str) {
    let seat = SEATS.lock().unwrap().remove(&live(app, key));
    let Some(s) = seat else { return };
    if s.variant == "B" {
        super::workfiles::back(&live(app, key), &s.tree);
    }
    super::fresh::session(app, key);
}
