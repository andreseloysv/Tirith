//! Which occurrence a question is about: a scheduled one by when it was due, or a routine's oldest in a state.
use super::store::{select, Expected};
use crate::app::App;
use rusqlite::params;

/// The scheduled occurrence of `routine` of `conv` due at `due`, when it was written down.
pub(super) fn occurrence(app: &App, (conv, routine, due): (&str, &str, &str)) -> Option<Expected> {
    let rest = "conv = ?1 AND routine = ?2 AND due = ?3 AND \"trigger\" = 'schedule' ORDER BY id";
    select(app, rest, params![conv, routine, due])
        .into_iter()
        .next()
}

/// True when that scheduled occurrence was written down already: it never runs, and is never said, twice.
pub fn known(app: &App, conv: &str, routine: &str, due: &str) -> bool {
    occurrence(app, (conv, routine, due)).is_some()
}

/// The oldest occurrence of that routine in `state`.
pub(super) fn oldest(app: &App, conv: &str, routine: &str, state: &str) -> Option<Expected> {
    let rest = "conv = ?1 AND routine = ?2 AND state = ?3 ORDER BY id LIMIT 1";
    select(app, rest, params![conv, routine, state])
        .into_iter()
        .next()
}
