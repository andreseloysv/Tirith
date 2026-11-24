//! Sign-ins under way, by their state (one use, ten minutes at most), and how each ended, by its ticket: what `neobot connector login` waits on. Held in memory alone: a restart ends them.
use super::token::Grant;
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const LIVES: Duration = Duration::from_secs(600);

pub(super) struct Pending {
    pub bot: String,
    pub id: String,
    pub verifier: String,
    pub redirect: String,
    pub grant: Grant,
    pub ticket: String,
    pub at: Instant,
}

static WAITING: Mutex<Vec<(String, Pending)>> = Mutex::new(Vec::new());
/// A ticket, how its sign-in ended, and when.
type Ended = (String, Result<(), String>, Instant);

static ENDED: Mutex<Vec<Ended>> = Mutex::new(Vec::new());

fn fresh(at: &Instant) -> bool {
    at.elapsed() < LIVES
}

pub(super) fn put(state: String, p: Pending) {
    let mut waiting = WAITING.lock().unwrap();
    waiting.retain(|(_, p)| fresh(&p.at));
    waiting.push((state, p));
}

/// The sign-in of `state`, taken: a second callback with it finds none.
pub(super) fn take(state: &str) -> Option<Pending> {
    let mut waiting = WAITING.lock().unwrap();
    let i = waiting
        .iter()
        .position(|(s, p)| s == state && fresh(&p.at))?;
    Some(waiting.remove(i).1)
}

pub(super) fn end(ticket: String, how: Result<(), String>) {
    let mut ended = ENDED.lock().unwrap();
    ended.retain(|(_, _, at)| fresh(at));
    ended.push((ticket, how, Instant::now()));
}

/// How the sign-in of `ticket` stands: waiting, done or failed with why; none for a ticket nobody has.
pub(super) fn status(ticket: &str) -> Option<Value> {
    let waiting = WAITING.lock().unwrap();
    if waiting
        .iter()
        .any(|(_, p)| p.ticket == ticket && fresh(&p.at))
    {
        return Some(json!({"status": "waiting"}));
    }
    let ended = ENDED.lock().unwrap();
    let (_, how, _) = ended.iter().find(|(t, _, _)| t == ticket)?;
    Some(match how {
        Ok(()) => json!({"status": "done"}),
        Err(why) => json!({"status": "failed", "why": why}),
    })
}
