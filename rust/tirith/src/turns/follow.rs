//! Which messages handed during a turn got turns of their own. The streaming CLI folds a line that comes during a turn into that turn (one result for both), or opens a turn for it right after that result (measured: a `system init` about 10 ms later).
use super::live::{open, with};
use crate::app::Shared;
use serde_json::Value;
use std::time::Duration;

/// With no turn opened this long after a result, what waited had joined the turn that ended.
const SETTLE: Duration = Duration::from_secs(3);

/// A line of the agent, before the turn reads it: one that begins work with no turn open begins the waiting message's turn.
pub fn follow(app: &Shared, conv: &str, msg: &Value) {
    let begins = match msg["type"].as_str() {
        Some("system") => msg["subtype"] == "init",
        Some("assistant" | "result") => true,
        _ => false,
    };
    let Some(()) = begins.then_some(()) else {
        return;
    };
    let next = with(conv, |c| match c.open {
        Some(_) => None,
        None => c.queued.pop_front().or_else(|| c.joined.take()),
    });
    if let Some(by) = next {
        open(app, conv, by);
    }
}

/// A turn ended with messages waiting: unless one gets its turn soon, they had joined it and the conversation is free.
pub(super) fn settle_later(app: &Shared, conv: &str, ends: u64) {
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        return;
    };
    let (app, conv) = (app.clone(), conv.to_string());
    runtime.spawn(async move {
        tokio::time::sleep(SETTLE).await;
        settle(&app, &conv, ends);
    });
}

/// The waiting messages had joined the turn that was the `ends`th to end: they were that one's, and nothing waits for them.
pub(super) fn settle(app: &Shared, conv: &str, ends: u64) {
    let joined = with(conv, |c| {
        let quiet = c.open.is_none() && c.ends == ends && !c.queued.is_empty();
        quiet.then(|| {
            c.joined = c.queued.pop_back(); // should its turn come late after all
            c.queued.clear();
        })
    });
    if joined.is_some() {
        super::capture_told::taken(conv); // the turn that ended was told these, not the next
        crate::manager::deliver(app, conv, "", false); // a routine's run may wait
    }
}

#[cfg(test)]
#[path = "../../checks/follow.rs"]
mod tests;
