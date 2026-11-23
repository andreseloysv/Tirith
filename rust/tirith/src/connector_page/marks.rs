//! A connector's real tools (those its last open listed) marked for one bot by the one verdict the gateway asks: `allowed`, `approval` or `denied`.
use crate::connectors::{verdict, Connector, Grant, Verdict};
use serde_json::{json, Value};

/// The mark of `tool` for a bot with grant `g` (None: not granted, or blocked for it alone).
pub fn mark(c: &Connector, g: Option<&Grant>, tool: &str) -> &'static str {
    match g.map(|g| verdict(c, g, tool)) {
        Some(Verdict::Run) => "allowed",
        Some(Verdict::Approval | Verdict::BotApproval) => "approval",
        _ => "denied",
    }
}

/// Each of `tools`, marked.
pub fn marks(c: &Connector, g: Option<&Grant>, tools: &[String]) -> Vec<Value> {
    let marked = |t: &String| json!({"name": t, "mark": mark(c, g, t)});
    tools.iter().map(marked).collect()
}
