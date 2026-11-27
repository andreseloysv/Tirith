//! Browser sign-ins under way: the state the provider sends back finds the PKCE verifier and where to go next.
//! Each lives ten minutes and is taken once; the state is kept as a hash, and only so many are kept at a time.
use crate::rooms::sealed::sha_hex;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const LIVES: Duration = Duration::from_secs(600);
const MOST: usize = 500;

pub(super) struct Entry {
    pub verifier: String,
    pub next: String,
    at: Instant,
}

static UNDER_WAY: Mutex<Vec<(String, Entry)>> = Mutex::new(Vec::new());

pub(super) fn add(state: &str, verifier: String, next: String) {
    let mut all = UNDER_WAY.lock().unwrap();
    all.retain(|(_, e)| e.at.elapsed() < LIVES);
    if all.len() >= MOST {
        all.remove(0);
    }
    let at = Instant::now();
    all.push((sha_hex(state.as_bytes()), Entry { verifier, next, at }));
}

/// The sign-in this state began, once, if it is not too old.
pub(super) fn take(state: &str) -> Option<Entry> {
    let wanted = sha_hex(state.as_bytes());
    let mut all = UNDER_WAY.lock().unwrap();
    let at = all.iter().position(|(s, _)| *s == wanted)?;
    Some(all.remove(at).1).filter(|e| e.at.elapsed() < LIVES)
}

#[cfg(test)]
pub(super) fn age(state: &str) {
    let wanted = sha_hex(state.as_bytes());
    for (_, e) in UNDER_WAY
        .lock()
        .unwrap()
        .iter_mut()
        .filter(|(s, _)| *s == wanted)
    {
        e.at = Instant::now() - LIVES - Duration::from_secs(1);
    }
}

/// Where to go once signed in: a path of this site, never another site's address.
pub(super) fn safe_next(next: &str) -> String {
    let own = next.starts_with('/')
        && !next.starts_with("//")
        && !next.starts_with("/\\")
        && !next.chars().any(char::is_control);
    if own {
        next.to_string()
    } else {
        "/".into()
    }
}
