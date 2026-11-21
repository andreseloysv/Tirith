//! A role's effective setting and where it comes from.
use crate::app::Shared;
use crate::config::{get, Config};
use serde::{Deserialize, Serialize};
/// The factory role that runs lanes (its `session = "lane"` one), by name.
pub fn lane_role(cfg: &Config) -> String {
    cfg.factory
        .find_role("lane", None)
        .unwrap_or_else(|| "worker".into())
}
#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RoleSetting {
    pub agent: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
    /// Next choices when the agent runs out of credits: "alias" or "alias:model", in order.
    #[serde(default)]
    pub fallback: Vec<String>,
}
/// The effective setting of a role and where it comes from: "page", "config" (config.toml's `[roles.<role>]` sets its agent, model, effort or fallback) or "default" (the factory's role alone, its `extends` resolved).
pub fn role_with_source(app: &Shared, role: &str) -> (RoleSetting, &'static str) {
    if let Some(s) = page_setting(app, role) {
        return (s, "page");
    }
    let cfg = app.cfg();
    if role.starts_with("bot-") {
        return (bot_setting(app, &cfg), "managers");
    }
    let source = if set_in_config(&cfg, role) {
        "config"
    } else {
        "default"
    };
    (factory_setting(&cfg, role), source)
}
/// The owner's override from the page, saved as `models:<role>`.
fn page_setting(app: &Shared, role: &str) -> Option<RoleSetting> {
    app.kv(&format!("models:{role}"))
        .and_then(|j| serde_json::from_str(&j).ok())
}
/// One of the owner's bots without a model of its own has the project chat role's (a system bot keeps its factory role's).
fn bot_setting(app: &Shared, cfg: &Config) -> RoleSetting {
    let chat = cfg
        .factory
        .find_role("chat", Some("project"))
        .unwrap_or_else(|| "manager".into());
    role(app, &chat)
}
/// True when config.toml's `[roles.<role>]` sets its agent, model, effort or fallback.
fn set_in_config(cfg: &Config, role: &str) -> bool {
    get::<toml::Table>(&cfg.raw, &["roles", role]).is_some_and(|t| {
        ["agent", "model", "effort", "fallback"].iter().any(|k| {
            t.get(*k)
                .is_some_and(|v| v.as_str().is_none_or(|s| !s.is_empty()))
        })
    })
}
/// The role as the factory (with config.toml laid over it) defines it; its agent defaults to the lane role's.
fn factory_setting(cfg: &Config, role: &str) -> RoleSetting {
    let f = &cfg.factory;
    let empty = |v: Option<String>| v.filter(|s| !s.is_empty());
    let agent = empty(f.role(role, "agent"))
        .or_else(|| empty(f.role(&lane_role(cfg), "agent")))
        .unwrap_or_else(|| "anthropic".into());
    RoleSetting {
        agent,
        model: empty(f.role(role, "model")),
        effort: empty(f.role(role, "effort")),
        fallback: f.role(role, "fallback").unwrap_or_default(),
    }
}
pub fn role(app: &Shared, role: &str) -> RoleSetting {
    role_with_source(app, role).0
}
