//! What the Connectors page shows: each connector (local or remote and its host, never its command, url path or secrets), blocked for all or not, its last use and real tools, and each bot granted it or blocked for it alone with its status, last use and the tools marked for it.
use super::{marks::marks, seen, status::status};
use crate::app::App;
use crate::connectors::Connector;
use serde_json::{json, Value};
use std::collections::HashMap;

/// When connector `id` was last called (by bot `bot`, when not empty).
fn last(app: &App, bot: &str, id: &str) -> String {
    let sql = "SELECT COALESCE(MAX(at), '') FROM connector_calls WHERE connector = ?1 AND (?2 = '' OR bot = ?2)";
    let at: Vec<String> = app.query(sql, [id, bot], |r| r.get(0));
    at.into_iter().next().unwrap_or_default()
}

/// A remote connector's host and port alone.
fn host(c: &Connector) -> String {
    let url = reqwest::Url::parse(&c.url).ok();
    let host = |u: &reqwest::Url| {
        Some(format!(
            "{}{}",
            u.host_str()?,
            u.port().map(|p| format!(":{p}")).unwrap_or_default()
        ))
    };
    url.as_ref().and_then(host).unwrap_or_default()
}

/// One connector as the page shows it; `names` holds each owner's bot's secret names.
fn one(app: &App, c: &Connector, bots: &[Value], names: &HashMap<String, Vec<String>>) -> Value {
    let (cfg, tools) = (app.cfg(), seen::tools(app, &c.id));
    let per: Vec<Value> = bots
        .iter()
        .filter_map(|b| {
            let (id, key) = (b["id"].as_str()?, b["key"].as_str()?);
            let grant = cfg
                .factory
                .granted(key)
                .into_iter()
                .find(|(x, _)| x.id == c.id);
            let alone = crate::access::blocked_for(app, id, &c.id);
            (grant.is_some() || alone).then_some(())?;
            let grant = grant.map(|(_, g)| g);
            let mut s = status(app, id, c, names.get(id).map_or(&[][..], Vec::as_slice));
            let (name, used) = (b["name"].clone(), last(app, id, &c.id));
            s.as_object_mut()?.extend([
                ("bot".into(), json!(id)),
                ("name".into(), name),
                ("blocked".into(), json!(alone)),
                ("last".into(), json!(used)),
                ("tools".into(), json!(marks(c, grant.as_ref(), &tools))),
            ]);
            Some(s)
        })
        .collect();
    let kind = if c.command.is_empty() {
        "remote"
    } else {
        "local"
    };
    json!({"id": c.id, "kind": kind, "host": host(c), "blocked": c.blocked, "last": last(app, "", &c.id), "tools": tools, "bots": per})
}

/// The page: every connector, and the owner's bots (id and name) a card may grant one to. It blocks: the bots' secret names are read from their store.
pub fn page(app: &crate::app::Shared) -> Value {
    let bots: Vec<Value> = crate::bots::all(app)
        .into_iter()
        .filter(|b| b["system"] != true)
        .collect();
    let names: HashMap<String, Vec<String>> = bots
        .iter()
        .filter_map(|b| b["id"].as_str())
        .map(|id| {
            (
                id.to_string(),
                crate::secrets::names(app, id).unwrap_or_default(),
            )
        })
        .collect();
    let connectors: Vec<Value> = app
        .cfg()
        .factory
        .connectors
        .iter()
        .map(|c| one(app, c, &bots, &names))
        .collect();
    let bots: Vec<Value> = bots
        .iter()
        .map(|b| json!({"id": b["id"], "name": b["name"]}))
        .collect();
    json!({"connectors": connectors, "bots": bots})
}
