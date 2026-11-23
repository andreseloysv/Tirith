//! The room's last messages, as the page shows them.
use super::{handle::Who, Msg, Room, KEPT};
use serde_json::Value;
/// A phone the owner had OpenBot forget: nothing it writes is read.
pub(super) fn blocked(r: &Room, id: &str) -> bool {
    r.managers.iter().any(|p| p.blocked && p.key_id == id)
}
/// Keeps the message (the last `KEPT`); the paired phone that wrote it, if one did.
pub(super) fn record(r: &mut Room, m: &Value, who: &Who, text: &str) -> Option<usize> {
    let device = r.devices.iter().position(|d| d.key_id == who.id);
    r.messages.push(Msg {
        n: m["n"].as_u64().unwrap_or(0),
        at: who.at.clone(),
        from: who.from.clone(),
        text: text.chars().take(2000).collect(),
        device: device.is_some(),
    });
    let _ = (r.messages.len() > KEPT).then(|| r.messages.remove(0));
    device
}
