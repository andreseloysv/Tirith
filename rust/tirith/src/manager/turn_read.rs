//! Reading a loose turn of a stream-json agent: its text, its usage and its permission requests.
use super::spawn::{control_reply, turn_permission};
use serde_json::Value;
use tokio::io::{AsyncWriteExt, BufReader, Lines};
use tokio::process::{ChildStdin, ChildStdout};

/// Reads the turn until its `result` (true) or the end of its output (false), gathering its text and answering its permission requests.
pub(super) async fn read_turn(
    lines: &mut Lines<BufReader<ChildStdout>>,
    stdin: &mut ChildStdin,
    read_only: bool,
    (text, rec): (&mut String, &mut crate::turns::Turn),
) -> bool {
    while let Ok(Some(line)) = lines.next_line().await {
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        rec.line(&msg);
        match msg["type"].as_str().unwrap_or("") {
            "assistant" => msg["message"]["content"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|b| b["type"] == "text")
                .for_each(|b| *text += &format!("{}\n", b["text"].as_str().unwrap_or(""))),
            "result" => return true,
            "control_request" if msg["request"]["subtype"] == "can_use_tool" => {
                let behavior = turn_permission(
                    read_only,
                    msg["request"]["tool_name"].as_str().unwrap_or(""),
                    &msg["request"]["input"],
                );
                let _ = stdin
                    .write_all(&super::turn::send(control_reply(&msg, behavior)))
                    .await;
            }
            _ => {}
        }
    }
    false
}
