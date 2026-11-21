//! Reading a stream-json session's output: its start, its words and tools, its turn ends and permission requests.
use super::blocks::assistant;
use super::spawn::control_reply;
use super::talk::{set_busy, talk};
use super::typing::{begin_turn, typing_stop};
use crate::app::Shared;
use crate::rooms;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStdout;
use tokio::sync::mpsc;
/// What reading a session needs: its conversation, the key its session id is kept under, and its stdin.
pub(super) struct Stream<'a> {
    pub(super) app: &'a Shared,
    pub(super) project: &'a str,
    pub(super) key: &'a str,
    pub(super) tx: &'a mpsc::UnboundedSender<String>,
}
/// Reads the session until its output ends or it rests: (it got ready, it rests).
pub(super) async fn read(s: &Stream<'_>, idle: Duration, stdout: ChildStdout) -> (bool, bool) {
    let (mut ready, mut rest) = (false, false);
    let mut lines = BufReader::new(stdout).lines();
    while !rest {
        let line = match tokio::time::timeout(idle, lines.next_line()).await {
            Ok(Ok(Some(line))) => line,
            Ok(_) => break,
            // Quiet for the role's idle_minutes and no turn running: the session rests until the next message resumes it.
            Err(_) => {
                rest = talk(s.project, |t| !t.busy && t.tx.take().is_some());
                continue;
            }
        };
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        match msg["type"].as_str().unwrap_or("") {
            "system" if msg["subtype"] == "init" => {
                ready = true;
                msg["session_id"]
                    .as_str()
                    .into_iter()
                    .for_each(|id| s.app.set_kv(s.key, Some(id)));
            }
            "assistant" => assistant(s.app, s.project, &msg),
            "result" => rest = turn_end(s.app, s.project),
            "control_request" if msg["request"]["subtype"] == "can_use_tool" => {
                // The rules already ran in the PreToolUse hook; anything that reaches here is allowed.
                let _ = s.tx.send(control_reply(
                    &msg,
                    json!({"behavior": "allow", "updatedInput": msg["request"]["input"]}),
                ));
            }
            _ => {}
        }
    }
    (ready, rest)
}
/// A turn ended: its answer goes to its rooms, and the next turn of this session starts. True when the session rests (the settings changed during the turn).
fn turn_end(app: &Shared, project: &str) -> bool {
    let (rest, rooms_now, text) = talk(project, |t| {
        t.phones = false;
        set_busy(app, project, t, false);
        // A message that comes after this waits in `pending` for the next session, which has the new settings.
        let rest = std::mem::take(&mut t.stale) && t.tx.take().is_some();
        (
            rest,
            std::mem::take(&mut t.turn_rooms),
            std::mem::take(&mut t.text),
        )
    });
    typing_stop(project);
    rooms::reply(app, &Vec::from_iter(rooms_now), &text);
    let _ = (!rest).then(|| begin_turn(app, project)); // what rooms sent during that turn is the next turn's, and in this session it starts now
    rest
}
