//! One `tools/call`: the verdict asked again with the grants of now (never the list the agent was given), a refusal answered without running, a call that asks first handed to its permit (`approve.rs`), a call that may run handed to its connector, and every attempt recorded.
use super::{approve::approval, names::split, record::record, rpc::failed, session::Session};
use crate::app::Shared;
use crate::connectors::{refusal, verdict, Verdict};
use serde_json::{json, Value};

/// The result the agent gets for calling `name` with `args`.
pub async fn call(app: &Shared, s: &Session, name: &str, args: &Value) -> Value {
    app.reload_config(false);
    let cfg = app.cfg();
    let ids: Vec<String> = cfg
        .factory
        .connectors
        .iter()
        .map(|c| c.id.clone())
        .collect();
    let (connector, tool) = split(name, &ids);
    let granted = cfg.factory.granted(&s.key);
    let granted = granted.iter().find(|(c, _)| c.id == connector);
    let link = s.links.iter().find(|l| l.id == connector);
    let v = granted.map_or(Verdict::OutOfScope, |(c, g)| verdict(c, g, tool));
    let said = match granted {
        _ if !v.refused() => None,
        Some((c, g)) => refusal(c, g, tool, &link.map(|l| l.names()).unwrap_or_default()),
        None => Some(format!(
            "{connector}.{tool}: refused by scope; this bot is not granted connector '{connector}'"
        )),
    };
    let (result, word, ran) = if let Some(said) = said {
        (failed(&said), v.word(), false)
    } else if matches!(v, Verdict::Approval | Verdict::BotApproval) {
        approval(app, s, (&connector, tool), args, link).await
    } else {
        let params = json!({"name": tool, "arguments": args});
        let result = match link {
            Some(l) => l
                .request("tools/call", params)
                .await
                .unwrap_or_else(|e| failed(&e)),
            None => failed(&format!(
                "connector '{connector}' is not open in this gateway"
            )),
        };
        (result, v.word(), true)
    };
    record(
        app,
        s,
        (&connector, tool),
        args,
        ran.then_some(&result),
        word,
    );
    result
}
