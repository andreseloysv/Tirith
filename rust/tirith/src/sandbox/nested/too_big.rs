//! A message past the bridge's cap is not carried: its call is answered that it failed, and the daemon never holds more.
use serde_json::{json, Value};

/// How much of a message's beginning can hold its id: Codex writes it first.
const HEAD: usize = 256;

/// Why it failed, for the model and for the caller.
pub(super) fn why() -> String {
    let most = super::frames::MOST >> 20;
    format!("Tirith stopped this call: it carries more than {most} MiB at once, the most a limited Codex's commands and patches pass. Work on it in smaller parts.")
}

/// The error that answers the call of which `start` is the beginning; none when it has no id (then nothing can be told so).
pub(super) fn refusal(start: &[u8]) -> Option<Vec<u8>> {
    let head = String::from_utf8_lossy(&start[..start.len().min(HEAD)]).into_owned();
    let (before, after) = head.split_once("\"id\":")?;
    // a message's own id, never one among its parameters
    (!before.contains(['[']) && before.matches('{').count() == 1).then_some(())?;
    let raw = after.split([',', '}']).next()?.trim();
    let id: Value = serde_json::from_str(raw).ok()?;
    (id.is_number() || id.is_string()).then_some(())?;
    let why = why();
    Some(
        json!({"id": id, "error": {"code": -32000, "message": why}})
            .to_string()
            .into_bytes(),
    )
}
