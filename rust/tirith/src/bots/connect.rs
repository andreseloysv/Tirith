//! A bot card's Connectors: which connectors the bot is granted and how narrowed, and a grant given, narrowed or removed through the bot store (`store::with`), which writes `[roles.bot-<id>.connectors.<connector>]` in config.toml (`grants.rs`), never a list of bots on the connector.
use super::store::{find, with};
use crate::app::Shared;
use crate::server::s;
use serde_json::{json, Value};

/// A list of plain globs, or why not.
fn globs(v: &Value, k: &str) -> Result<Vec<String>, String> {
    let bad = || format!("{k} is a list of non-empty globs");
    let list = v.as_array().map(Vec::as_slice).unwrap_or_default();
    (v.is_null() || v.is_array())
        .then_some(())
        .ok_or_else(bad)?;
    list.iter()
        .map(|x| {
            x.as_str()
                .map(str::trim)
                .filter(|x| !x.is_empty() && !x.contains(['\n', '"']))
                .map(String::from)
                .ok_or_else(bad)
        })
        .collect()
}

/// The grant `g` asks for, as the writer lays it: `tools` (null: all), `deny` and `approval`.
fn grant(g: &Value) -> Result<Value, String> {
    let tools = (!g["tools"].is_null())
        .then(|| globs(&g["tools"], "tools"))
        .transpose()?;
    Ok(
        json!({"tools": tools, "deny": globs(&g["deny"], "deny")?, "approval": globs(&g["approval"], "approval")?}),
    )
}

/// Gives (`g` an object), narrows or removes (`g` null) bot `id`'s grant of `connector`.
pub fn set(app: &Shared, id: &str, connector: &str, g: &Value) -> Result<Value, String> {
    app.cfg()
        .factory
        .connector(connector)
        .ok_or_else(|| format!("no connector '{connector}' in config.toml"))?;
    if crate::access::blocked_for(app, id, connector) {
        return Err(format!("connector {connector} is blocked for bot {id}: unblock it on the Connectors page first"));
    }
    let table = if g.is_null() { Value::Null } else { grant(g)? };
    with(app, |bots| {
        let bot = find(bots, id)?;
        (bot["system"] != true)
            .then_some(())
            .ok_or("a system bot holds no connector")?;
        bot["grant"] = json!({"connector": connector, "table": table});
        Ok(())
    })?;
    granted(app, id)
}

/// Each connector as the bot's card shows it: granted or not, blocked (for all or for it alone) and its narrowing.
pub fn granted(app: &Shared, id: &str) -> Result<Value, String> {
    let key = super::key_for(app, id)
        .filter(|k| k.starts_with("bot-"))
        .ok_or_else(|| format!("no bot '{id}' of the owner's"))?;
    let cfg = app.cfg();
    let mine = cfg.factory.granted(&key);
    let rows = cfg.factory.connectors.iter().map(|c| {
        let g = mine.iter().find(|(x, _)| x.id == c.id).map(|(_, g)| g);
        let alone = crate::access::blocked_for(app, id, &c.id);
        json!({"id": c.id, "granted": g.is_some(), "blocked": c.blocked || alone, "tools": g.and_then(|g| g.tools.clone()), "deny": g.map(|g| g.deny.clone()).unwrap_or_default(), "approval": g.map(|g| g.approval.clone()).unwrap_or_default()})
    });
    Ok(json!(rows.collect::<Vec<_>>()))
}

/// The card's body: `{connector, grant}`.
pub fn posted(app: &Shared, id: &str, b: &Value) -> Result<Value, String> {
    set(app, id, s(b, "connector").trim(), &b["grant"])
}
