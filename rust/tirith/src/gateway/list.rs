//! What the agent is listed: each open connector's tools the verdict lets this bot reach now, under `<connector>__<tool>`; one that asks first says so in its description.
use super::{names::joined, session::Session};
use crate::app::Shared;
use crate::connectors::{verdict, Verdict};
use serde_json::Value;

/// The tools of `s` as `tools/list` gives them, by the grants of now.
pub fn tools(app: &Shared, s: &Session) -> Vec<Value> {
    app.reload_config(false);
    let cfg = app.cfg();
    let granted = cfg.factory.granted(&s.key);
    let mut out = vec![];
    for link in &s.links {
        let Some((c, g)) = granted.iter().find(|(c, _)| c.id == link.id) else {
            continue;
        };
        for tool in &link.catalog {
            let name = tool["name"].as_str().unwrap_or("");
            let v = verdict(c, g, name);
            if v.refused() {
                continue;
            }
            let mut shown = tool.clone();
            shown["name"] = joined(&c.id, name).into();
            if matches!(v, Verdict::Approval | Verdict::BotApproval) {
                let said = tool["description"].as_str().unwrap_or("");
                shown["description"] = format!("(asks the owner first) {said}").into();
            }
            out.push(shown);
        }
    }
    out
}
