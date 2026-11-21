//! A codex lane's session: the thread id its current run started, kept on the lane so it can resume.
use super::logscan::RUN_START;
use crate::app::Shared;

pub(super) fn codex_session(app: &Shared, id: &str, log: &str) {
    let run = log.rsplit(RUN_START).next().unwrap_or(log);
    let session = run
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|v| v["type"] == "thread.started")
        .filter_map(|v| v["thread_id"].as_str().map(String::from))
        .next_back();
    session
        .filter(|s| app.lane(id).is_some_and(|l| l.session != *s))
        .into_iter()
        .for_each(|s| drop(app.update_lane(id, |l| l.session = s)));
}
