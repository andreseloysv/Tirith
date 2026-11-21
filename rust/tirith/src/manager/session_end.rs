//! The end of a stream-json session: what its dead turn left, its process, and what waits again when it never got ready.
use super::talk::talk;
use crate::app::{kill_group, Shared};
use crate::rooms;
use std::time::Duration;
use tokio::process::Child;
use tokio::task::JoinHandle;
/// A running session's process and the task writing to it.
pub(super) struct Live {
    pub(super) child: Child,
    pub(super) pid: u32,
    pub(super) writer: JoinHandle<()>,
}
/// Ends the session's process; a turn it left unanswered hands its rooms' messages over again.
pub(super) async fn close(app: &Shared, project: &str, mut live: Live, ready: bool) {
    // What a dead turn left is never sent to a room with a later answer; what rooms wrote for it is handed over again, once, and before the wait below, so a line that arrives during it (and waits in `pending`) is not handed twice.
    let died = talk(project, |t| {
        (t.tx.take().is_some() && t.busy || !t.turn_rooms.is_empty()) && ready
    }); // a session that never got ready ran no turn: its lines wait, with their rooms; a promoted turn it never answered counts as died too
    let _ = died.then(|| {
        (
            talk(project, |t| (t.reply_to.clear(), t.turn_rooms.clear())),
            rooms::again(app, Some(project)),
        )
    });
    live.writer.abort();
    kill_group(live.pid, libc::SIGTERM);
    let _ = tokio::time::timeout(Duration::from_secs(10), live.child.wait()).await;
}
/// It ended before it was ready: what it was handed waits again. A resume that does not work becomes a fresh session.
pub(super) fn not_ready(
    app: &Shared,
    project: &str,
    key: &str,
    sent: Vec<String>,
    resumed: bool,
) -> Result<(), String> {
    talk(project, |t| drop(t.pending.splice(0..0, sent)));
    let Some(()) = resumed.then_some(()) else {
        return Err("its agent ended before it was ready (logs/manager.log says why)".into());
    };
    app.set_kv(key, None);
    Ok(())
}
