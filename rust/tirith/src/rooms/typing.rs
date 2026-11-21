//! The phones see a turn typing, and its tools, while it runs.
use super::{outbox::queue, store::room};
use serde_json::{json, Value};
/// One turn's live progress for the phones in its rooms, as `tirith-chat` controls a phone page that does not know them ignores: typing on at the start, at most one short tool line every 5 s (never the same twice in a row, 120 chars at most), its current state resent as a heartbeat when 30 s passed since the last control (so a long command without lines never looks stopped and a reopened phone sees it again), typing off at the end. Another message for the running turn, or a new room for it, reannounces that same state without resetting its line or repeating its start. Each call says what to send, to the turn's rooms alone; no rooms, nothing. It is no message: `handle` drops `tirith-chat`, so it keeps no history, counts no streak and wakes no phone like an answer does.
#[derive(Default)]
pub(crate) struct Typing {
    rooms: Vec<String>,
    last: String,
    at: Option<u64>,
    sent: Option<u64>,
}
impl Typing {
    fn current(&self) -> (Vec<String>, Value) {
        (
            self.rooms.clone(),
            if self.last.is_empty() {
                json!({"v": 1, "typing": 1})
            } else {
                json!({"v": 1, "typing": 1, "line": self.last})
            },
        )
    }
    /// The turn's start, in `rooms`; with `again`, another message for a turn that already runs. A running turn takes in any room it gains and shows its current state again only for such a room or with `again`: nobody sees a second start, and its line is never reset.
    pub(crate) fn start(
        &mut self,
        rooms: Vec<String>,
        now: u64,
        again: bool,
    ) -> Option<(Vec<String>, Value)> {
        if self.rooms.is_empty() {
            (!rooms.is_empty()).then_some(())?;
            *self = Typing {
                rooms,
                sent: Some(now),
                ..Default::default()
            };
            return Some(self.current());
        }
        let new: Vec<String> = rooms
            .into_iter()
            .filter(|r| !self.rooms.contains(r))
            .collect();
        let added = !new.is_empty();
        self.rooms.extend(new);
        (again || added).then(|| (self.sent = Some(now), self.current()).1)
    }
    pub(crate) fn heartbeat(&mut self, now: u64) -> Option<(Vec<String>, Value)> {
        (!self.rooms.is_empty() && self.sent.is_none_or(|sent| now.saturating_sub(sent) >= 30))
            .then_some(())?;
        self.sent = Some(now);
        Some(self.current())
    }
    pub(crate) fn line(&mut self, line: &str, now: u64) -> Option<(Vec<String>, Value)> {
        let line: String = line.trim().chars().take(120).collect();
        (!self.rooms.is_empty()
            && !line.is_empty()
            && line != self.last
            && self.at.is_none_or(|at| now.saturating_sub(at) >= 5))
        .then_some(())?;
        (self.last, self.at, self.sent) = (line, Some(now), Some(now));
        Some(self.current())
    }
    pub(crate) fn stop(&mut self) -> Option<(Vec<String>, Value)> {
        let rooms = std::mem::take(&mut self.rooms);
        (!rooms.is_empty()).then(|| (rooms, json!({"v": 1, "typing": 0})))
    }
}
/// Sends one turn's `tirith-chat` control to its rooms alone, sealed for whoever is in them (their phones and accepted managers).
pub fn typing(sent: Option<(Vec<String>, Value)>) {
    let Some((rooms, body)) = sent else { return };
    let text = format!("tirith-chat {body}");
    rooms
        .iter()
        .filter_map(|id| room(id).map(|r| (id, r)))
        .for_each(|(id, r)| {
            queue(
                id,
                &text,
                r.devices
                    .iter()
                    .chain(r.managers.iter().filter(|m| m.accepted))
                    .map(|p| p.public.clone())
                    .collect(),
            )
        });
}
