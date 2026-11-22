//! Lines that must reach the phones (a routine that failed): kept (kv `rooms:unsent`) until sent, so a restart while the platform is away loses none.
use super::outbox::{queue, Line};
use crate::app::App;
use std::sync::Mutex;

const KEY: &str = "rooms:unsent";
const MOST: usize = 100;
static CHANGING: Mutex<()> = Mutex::new(());

fn kept(app: &App) -> Vec<Line> {
    app.kv(KEY)
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default()
}

fn change(app: &App, f: impl FnOnce(&mut Vec<Line>)) {
    let _one = CHANGING.lock().unwrap_or_else(|e| e.into_inner());
    let mut lines = kept(app);
    let before = lines.clone();
    f(&mut lines);
    let from = lines.len().saturating_sub(MOST);
    lines.drain(..from);
    if lines != before {
        let text = serde_json::to_string(&lines).unwrap_or_default();
        app.set_kv(KEY, (!lines.is_empty()).then_some(text.as_str()));
    }
}

/// `line` is owed to the phones.
pub(super) fn keep(app: &App, line: Line) {
    change(app, |lines| lines.push(line));
}

/// `(room, text)` was sent, or was given up and said.
pub(super) fn done(app: &App, room: &str, text: &str) {
    change(app, |lines| {
        if let Some(at) = lines.iter().position(|l| l.0 == room && l.1 == text) {
            lines.remove(at);
        }
    });
}

/// At the start: what was still owed is queued again.
pub(super) fn again(app: &App) {
    for (room, text, readers) in kept(app) {
        queue(&room, &text, readers);
    }
}
