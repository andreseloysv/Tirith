//! What GET /api/models gives the page.
use super::{owners::agent_of, roles::role_with_source};
use crate::app::Shared;
use crate::config::{get, Config};
use serde_json::{json, Map, Value};
const EFFORTS: [&str; 6] = ["low", "medium", "high", "xhigh", "max", "ultra"];
/// GET /api/models: every factory role's effective setting (the bots among them) with its source and its factory shape (session, scope, access, runs_on, name, face, show), the choices the page offers, and the closed list of events a bot's card offers its routes from (but `schedule`: its routines).
pub fn state(app: &Shared) -> Value {
    let cfg = app.cfg();
    let roles: Map<String, Value> = cfg
        .factory
        .roles()
        .into_iter()
        .map(|r| {
            let v = role_view(app, &cfg, &r);
            (r, v)
        })
        .collect();
    let models = listed_models(&cfg);
    let owners: Map<String, Value> = models
        .iter()
        .cloned()
        .filter_map(|m| agent_of(&cfg, &m).map(|a| (m, json!(a))))
        .collect();
    json!({"roles": roles, "events": crate::factory::EVENTS.split(';').filter(|e| !matches!(*e, "schedule" | "webhook")).collect::<Vec<_>>(), "choices": {"agents": cfg.factory.commands().iter().map(|(k, _)| k).collect::<Vec<_>>(), "codex": crate::codex::catalog(), "models": &models, "owners": owners,
        "efforts": get::<Vec<String>>(&cfg.raw, &["models", "efforts"]).unwrap_or_else(|| EFFORTS.map(String::from).to_vec())}})
}
/// A role's setting with its source and its factory shape.
fn role_view(app: &Shared, cfg: &Config, r: &str) -> Value {
    let (s, source) = role_with_source(app, r);
    let mut v = json!(s);
    v["source"] = json!(source);
    ["session", "scope", "access", "runs_on", "name"]
        .into_iter()
        .filter_map(|k| cfg.factory.role::<String>(r, k).map(|x| (k, x)))
        .for_each(|(k, x)| v[k] = json!(x));
    v["show"] = json!(cfg.factory.role::<bool>(r, "show"));
    let face = |k: &str| get::<String>(&cfg.factory.raw, &["roles", r, "face", k]);
    v["face"] = json!({"shape": face("shape"), "color": face("color"), "material": face("material"), "eye": face("eye"), "gradient": face("gradient"), "gstart": face("gstart"), "gend": face("gend"), "glass": face("glass"), "gangle": get::<f64>(&cfg.factory.raw, &["roles", r, "face", "gangle"])});
    v
}
/// The models of `[models.choices]` (a list, or a table of lists per agent), then the catalog's models of each
/// configured agent that list names none of: a new install offers its agents' models before anyone lists them.
fn listed_models(cfg: &Config) -> Vec<String> {
    let listed = get::<toml::Value>(&cfg.raw, &["models", "choices"]).map(|c| {
        c.as_table()
            .map_or(vec![c.clone()], |t| t.values().cloned().collect())
    });
    let mut all: Vec<String> = listed
        .into_iter()
        .flatten()
        .filter_map(|v| v.try_into::<Vec<String>>().ok())
        .flatten()
        .collect();
    for (alias, _) in cfg.factory.commands() {
        let has = all
            .iter()
            .any(|m| agent_of(cfg, m).as_deref() == Some(alias.as_str()));
        let known = crate::setup::known_agent(&alias).map(|k| k.models);
        all.extend(known.into_iter().flatten().filter(|_| !has));
    }
    all
}
