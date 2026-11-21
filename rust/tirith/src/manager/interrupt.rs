//! Stopping the turn a conversation runs now.
use super::talk::talk;
use crate::app::kill_group;
use serde_json::json;
/// Stops the current turn of the conversation's session (the session lives on): a stream-json `control_request` with subtype `interrupt`, as the old collector sent. Err when no turn is running.
pub fn interrupt(project: &str) -> Result<(), String> {
    // A busy turn without a live session's stdin is a one-shot exec run (Codex, Muse): interrupting it means killing it, not a control line.
    let Some(()) = (!talk(project, |t| {
        (t.busy && t.tx.is_none())
            .then(|| kill_group(t.pid, libc::SIGTERM))
            .is_some()
    }))
    .then_some(()) else {
        return Ok(());
    };
    let tx = talk(project, |t| t.tx.clone().filter(|_| t.busy))
        .ok_or("the manager is not working on a turn")?;
    tx.send(json!({"type": "control_request", "request_id": format!("int-{:08x}", rand::random::<u32>()), "request": {"subtype": "interrupt"}}).to_string()).map_err(|_| "the manager session ended".to_string())
}
