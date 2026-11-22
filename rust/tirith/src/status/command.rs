//! The owner's `/status` command and the answer it gives.
use super::{error::last_error, lanes::lanes, routines::routines, span::span};
use crate::app::Shared;
use chrono::{DateTime, Local};
/// True when the owner's message is `/status`.
pub fn asked(text: &str) -> bool {
    text.split_whitespace().next() == Some("/status")
}
/// The answer to `/status`, if `text` is one: no model is asked.
pub fn command(app: &Shared, text: &str) -> Option<String> {
    asked(text).then(|| report(app, Local::now()))
}
/// The four facts, as they are at `now`.
pub(super) fn report(app: &Shared, now: DateTime<Local>) -> String {
    let up = span((now.timestamp() as u64).saturating_sub(app.started));
    format!(
        "Up for {up}\n\n{}\n\n{}\n\n{}",
        lanes(app),
        routines(app, now),
        last_error(app, now)
    )
}
