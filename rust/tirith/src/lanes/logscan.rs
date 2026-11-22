//! Reading a lane's agent log: how long it has been quiet, whether its run ended, and its last commands.
use std::collections::HashMap;

/// Seconds since the agent's log last got a line other than an `api_retry` one (those keep the file fresh while the agent is stuck). `seen` remembers per lane the last real line and when it appeared.
pub(super) fn quiet_s(
    seen: &mut HashMap<String, (String, u64)>,
    id: &str,
    tail: &str,
    log_age: u64,
    now: u64,
) -> u64 {
    let last = tail
        .lines()
        .rev()
        .find(|l| !l.contains("\"api_retry\""))
        .unwrap_or("");
    let e = seen
        .entry(id.to_string())
        .or_insert((last.to_string(), now.saturating_sub(log_age)));
    let _ = (e.0 != last).then(|| *e = (last.to_string(), now));
    now.saturating_sub(e.1)
}
/// Written into a lane's log at every launch: the log is appended across attempts, so `run_completed` only counts a terminal after the last launch line (or, for a run started before these lines existed, after its last `run.lifecycle.started`).
pub(super) const RUN_START: &str = "--- tirith: lane run started ";
/// What the current run wrote: the log goes on across runs, and what an agent before this one said (out of credits) is not this one's.
pub(super) fn this_run(tail: &str) -> &str {
    tail.rfind(RUN_START).map_or(tail, |at| &tail[at..])
}
/// Whether the lane's current run already ended in its log: a `run.terminal.completed` event (the top-level `payload_type` of its own stream line, so a mention inside a file or command output the agent read never counts). The process may still be alive.
pub(super) fn run_completed(tail: &str) -> bool {
    tail.lines().fold(false, |done, line| {
        match serde_json::from_str::<serde_json::Value>(line)
            .ok()
            .and_then(|v| v["payload_type"].as_str().map(String::from))
            .as_deref()
        {
            Some("run.lifecycle.started") => false,
            Some("run.terminal.completed") => true,
            _ if line.starts_with(RUN_START) => false,
            _ => done,
        }
    })
}
/// The last `n` shell commands the agent ran, from the tail of its stream-json log (oldest first).
pub(super) fn last_commands(tail: &str, n: usize) -> Vec<String> {
    let mut cmds: Vec<String> = tail
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .flat_map(|v| {
            v["message"]["content"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .filter_map(|c| {
            c["input"]["command"]
                .as_str()
                .map(|s| s.chars().take(200).collect::<String>())
        })
        .collect();
    cmds.drain(..cmds.len().saturating_sub(n));
    cmds
}
