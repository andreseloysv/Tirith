//! A capture's calls that read outside, as a replay answers them.
use super::call_key::key;
use super::outside::outside;
use crate::server::s;
use serde_json::Value;

/// One outside call of the capture: its tool as its agent named it, what it asked for, its result, and whether a call of the replay took it.
pub(super) struct Call {
    pub tool: String,
    pub key: String,
    pub result: String,
    pub used: bool,
}

/// A captured call under the rules' names.
fn norm(c: &Value) -> (String, Value) {
    crate::rules::normalize(&s(c, "tool"), &c["input"], "")
}

/// The calls of `calls` that read outside, waiting to be asked for again.
pub(super) fn reads(calls: &[Value]) -> Vec<Call> {
    let out = calls.iter().map(|c| (c, norm(c)));
    out.filter(|(_, (tool, input))| outside(tool, input))
        .map(|(c, (tool, input))| Call {
            tool: s(c, "tool"),
            key: key(&tool, &input),
            result: s(c, "result"),
            used: false,
        })
        .collect()
}

/// True when each outside read of a capture has the result to answer it with (a server's own web search keeps none).
pub(super) fn answerable(calls: &[Value]) -> bool {
    calls.iter().all(|c| {
        let (tool, input) = norm(c);
        c.get("result").is_some() || !outside(&tool, &input)
    })
}
