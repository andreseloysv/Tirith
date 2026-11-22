//! The scheduler's tick: Tirith says it is alive, what was missed is written down, what takes too long is ended, a run no judge will read is looked at by code, and only then do news that waited for that judge go out (after a stop both are due at once, and a failure is one notice).
use crate::app::Shared;
use chrono::{DateTime, Local};

/// Before the routines due now are fired, so a late one knows why it is late.
pub fn tick(app: &Shared, now: DateTime<Local>) {
    super::gaps::beat(app, now);
    super::missed::check(app, now);
    super::overdue::check(app, now);
    super::unjudged::check(app, now);
    super::news::overdue(app, now);
}
