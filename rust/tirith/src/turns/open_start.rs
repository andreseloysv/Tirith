//! The turn of a conversation open now, for what is recorded against it from outside its lines (`gateway/`).

/// The start (µs since the epoch) of the turn of `conv` open now; 0 when none is.
pub fn open_start(conv: &str) -> i64 {
    super::live::with(conv, |c| c.open.as_ref().map_or(0, |t| t.start_us))
}
