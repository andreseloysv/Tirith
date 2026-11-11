//! Models per role and the fallback chain.
//!
//! Roles: "manager" (the chat session), "workers" (lanes), "ceo" (one-shot review), "brain" (small text jobs
//! through the model API: only its model matters). Each has an agent alias, a model, an effort and a fallback
//! chain. The effective setting is the owner's override from the page (kv `models:<role>`), else
//! `[models.<role>]` of `config.toml`, else a default. Agents are aliases (`[agents.<alias>]`); the real command
//! lives only in config.toml. Optional per alias: `model_flag`, `effort_flag` ("" = none) and `resume_args`
//! (`{session}` and `{message}` are filled in); the known aliases have defaults.

use crate::app::{bad, Shared};
use crate::client::{call, Flag};
use crate::config::{get, Config};
use axum::extract::{Path as UrlPath, State};
use axum::routing::get as GET;
use axum::{Json, Router};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::OnceLock;

pub const ROLES: [&str; 4] = ["manager", "workers", "ceo", "brain"];
const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];
pub const BRAIN_MODEL: &str = "muse-spark-1.3";
const PROMPT: &str = "Read the brief at {brief} and do it completely in {workspace}.";

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

/// An agent command line: the alias it belongs to and the full argv (program first).
pub struct AgentCmd {
    pub alias: String,
    pub argv: Vec<String>,
}

/// The effective setting of a role and where it comes from: "page", "config" or "default".
pub fn role_with_source(app: &Shared, role: &str) -> (RoleSetting, &'static str) {
    if let Some(s) = app.kv(&format!("models:{role}")).and_then(|j| serde_json::from_str(&j).ok()) {
        return (s, "page");
    }
    let cfg = app.cfg();
    let default_agent = || if role == "workers" { "anthropic".to_string() } else { role_with_source(app, "workers").0.agent };
    let table: Option<toml::Table> = get(&cfg.raw, &["models", role]).or_else(|| (role == "ceo").then(|| get(&cfg.raw, &["ceo"])).flatten());
    let s = |t: &toml::Table, k: &str| t.get(k).and_then(|v| v.as_str()).map(String::from).filter(|v| !v.is_empty());
    match table {
        Some(t) if role != "ceo" || t.contains_key("model") || t.contains_key("agent") => (
            RoleSetting {
                agent: if role == "brain" { String::new() } else { s(&t, "agent").unwrap_or_else(default_agent) },
                model: s(&t, "model").or_else(|| (role == "brain").then(|| BRAIN_MODEL.into())),
                effort: s(&t, "effort"),
                fallback: get(&cfg.raw, &["models", role, "fallback"]).unwrap_or_default(),
            },
            "config",
        ),
        _ if role == "brain" => (RoleSetting { model: Some(BRAIN_MODEL.into()), ..Default::default() }, "default"),
        _ => (RoleSetting { agent: default_agent(), ..Default::default() }, "default"),
    }
}

/// The effective setting of a role.
pub fn role(app: &Shared, role: &str) -> RoleSetting {
    role_with_source(app, role).0
}

/// Codex must not apply its own sandbox: the lane already runs inside the daemon's `sandbox-exec`, and a nested one
/// fails every command with "sandbox_apply: Operation not permitted".
const NO_SANDBOX: &str = "--dangerously-bypass-approvals-and-sandbox";

/// Per alias: (model flag, effort flag, resume args, lane args), from `[agents.<alias>]` or the known defaults.
fn flags(cfg: &Config, alias: &str) -> (Option<String>, Option<String>, Vec<String>, Vec<String>) {
    let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let (m, e, resume, args): (&str, &str, Vec<String>, Vec<String>) = match alias {
        "anthropic" => ("--model", "--effort", v(&["--resume", "{session}", "-p", "{message}"]), v(&["-p", PROMPT])),
        "muse" => ("--model", "--reasoning-effort", vec![], v(&["exec", "--json", "--workspace", "{workspace}", "--prompt-file", "{brief}"])),
        "codex" => ("--model", "", v(&["exec", "--json", NO_SANDBOX, "resume", "{session}", "{message}"]), v(&["exec", "--json", NO_SANDBOX, PROMPT])),
        "copilot" => ("--model", "--reasoning-effort", v(&["--resume", "{session}", "-p", "{message}"]), v(&["--output-format", "json", "--no-ask-user", "--allow-tool", "write", "--allow-tool", "shell", "-p", PROMPT])),
        _ => ("--model", "", v(&["--resume", "{session}", "-p", "{message}"]), v(&["-p", PROMPT])),
    };
    let key = |k: &str, d: &str| Some(get::<String>(&cfg.raw, &["agents", alias, k]).unwrap_or(d.into())).filter(|f| !f.is_empty());
    let resume = get(&cfg.raw, &["agents", alias, "resume_args"]).unwrap_or(resume);
    let args = cfg.agents.get(alias).and_then(|a| a.args.clone()).unwrap_or(args);
    (key("model_flag", m), key("effort_flag", e), resume, args)
}

/// Sets `flag value` in argv: replaces the value of an existing `flag`/`alt` (also `flag=value`), else appends.
fn set_flag(argv: &mut Vec<String>, flag: &Option<String>, alts: &[&str], value: &Option<String>) {
    let (Some(flag), Some(value)) = (flag, value) else { return };
    let is = |w: &str| w == flag || alts.contains(&w);
    if let Some(i) = argv.iter().position(|w| is(w)).filter(|i| i + 1 < argv.len()) {
        argv[i + 1] = value.clone();
    } else if let Some(w) = argv.iter_mut().find(|w| w.split_once('=').is_some_and(|(f, _)| is(f))) {
        *w = format!("{}={value}", w.split_once('=').unwrap().0);
    } else {
        argv.extend([flag.clone(), value.clone()]);
    }
}

/// The argv of a lane run. `agent`/`model` override the workers role (a fallback relaunch passes them).
/// `resume = Some((session_id, message))` continues that agent session with `message` instead of the brief;
/// an alias without `resume_args` starts fresh with the brief.
pub fn lane_argv(app: &Shared, agent: Option<&str>, model: Option<&str>, brief: &Path, workspace: &Path, resume: Option<(&str, &str)>) -> Result<AgentCmd, String> {
    let cfg = app.cfg();
    let w = role(app, "workers");
    let alias = agent.unwrap_or(&w.agent).to_string();
    let a = cfg.agents.get(&alias).ok_or(format!("no agent alias '{alias}' in config.toml"))?;
    let same = alias == w.agent;
    let model = model.map(String::from).or(if same { w.model } else { None });
    let effort = if same { w.effort } else { None };
    let (mflag, eflag, resume_args, args) = flags(&cfg, &alias);
    let (brief, ws) = (brief.display().to_string(), workspace.display().to_string());
    let (session, message) = resume.unwrap_or_default();
    let fill = |s: &String| s.replace("{brief}", &brief).replace("{workspace}", &ws).replace("{session}", session).replace("{message}", message);
    let mut argv: Vec<String> = if resume.is_some() && !resume_args.is_empty() {
        std::iter::once(a.command.clone()).chain(resume_args.iter().map(fill)).collect()
    } else if Some(alias.as_str()) == cfg.workers_agent.as_deref().or(Some("anthropic")) && !cfg.launch.is_empty() {
        cfg.launch.iter().map(fill).collect()
    } else {
        std::iter::once(a.command.clone()).chain(args.iter().map(fill)).collect()
    };
    set_flag(&mut argv, &mflag, &["--model", "-m"], &model);
    set_flag(&mut argv, &eflag, &["--effort", "--reasoning-effort"], &effort);
    Ok(AgentCmd { alias, argv })
}

/// The argv of a streaming chat session (stream-json in and out) for "manager" or "ceo".
/// `resume` continues that session id. The caller appends its own system prompt.
pub fn chat_argv(app: &Shared, role_name: &str, resume: Option<&str>) -> Result<AgentCmd, String> {
    let cfg = app.cfg();
    let r = role(app, role_name);
    let a = cfg.agents.get(&r.agent).ok_or(format!("no agent alias '{}' in config.toml", r.agent))?;
    let mut argv = vec![a.command.clone()];
    argv.extend(a.chat_args.clone().unwrap_or_else(|| ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose"].map(String::from).to_vec()));
    let (mflag, eflag, _, _) = flags(&cfg, &r.agent);
    set_flag(&mut argv, &mflag, &["--model", "-m"], &r.model);
    set_flag(&mut argv, &eflag, &["--effort", "--reasoning-effort"], &r.effort);
    if let Some(id) = resume {
        argv.extend(["--resume".into(), id.into()]);
    }
    Ok(AgentCmd { alias: r.agent, argv })
}

/// True when an agent's output says it ran out of credits or hit its usage limit.
pub fn out_of_credits(text: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(concat!(
            r"(?i)credit balance is too low|insufficient[ _-]?(credits?|quota|funds|balance)|out of credits",
            r"|no credits? (left|remaining)|(usage|spending|session|weekly|daily|monthly|[0-9]+-hour) limit (reached|exceeded|hit)",
            r"|(hit|reached|exceeded) (your|the) (usage |spending |current |monthly )?(limit|quota)|quota (exceeded|exhausted)",
            r"|resource[_ ]exhausted|payment required|\b402\b[^\n]{0,40}(payment|credit|billing)|run out of (credits|usage|quota)",
        ))
        .unwrap()
    })
    .is_match(text)
}

/// The next choice of a role's chain (its own agent, then `fallback`) that is not in `tried`: (alias, model).
/// `tried` holds "alias" (every model of it is skipped) or "alias:model" (that one only).
pub fn fallback(app: &Shared, role_name: &str, tried: &[String]) -> Option<(String, Option<String>)> {
    let cfg = app.cfg();
    let r = role(app, role_name);
    let first = r.model.as_ref().map(|m| format!("{}:{m}", r.agent)).unwrap_or(r.agent.clone());
    std::iter::once(first).chain(r.fallback).find_map(|e| {
        let (alias, model) = e.split_once(':').map(|(a, m)| (a.to_string(), Some(m.to_string()))).unwrap_or((e.clone(), None));
        let used = tried.iter().any(|t| *t == e || *t == alias);
        (!used && cfg.agents.contains_key(&alias)).then_some((alias, model))
    })
}

fn valid(v: &str) -> bool {
    v.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()) && v.chars().all(|c| c.is_ascii_alphanumeric() || "._:/+-[]".contains(c))
}

/// Saves the owner's setting of a role (fields absent from `b` keep their effective value, "" clears them),
/// or with `None` goes back to config.toml. Restarts the manager session when its setting changed.
pub fn save(app: &Shared, role_name: &str, b: Option<&Value>) -> Result<RoleSetting, String> {
    if !ROLES.contains(&role_name) {
        return Err(format!("role must be one of {}", ROLES.join(", ")));
    }
    let before = role(app, "manager");
    if let Some(b) = b {
        let mut s = role(app, role_name);
        let field = |k: &str, cur: Option<String>| match &b[k] {
            Value::String(v) if v.trim().is_empty() => None,
            Value::String(v) => Some(v.trim().to_string()),
            _ => cur,
        };
        if role_name != "brain" {
            s.agent = field("agent", Some(s.agent.clone())).unwrap_or_default();
        }
        s.model = field("model", s.model);
        s.effort = field("effort", s.effort);
        match &b["fallback"] {
            Value::Array(a) => s.fallback = a.iter().filter_map(|x| x.as_str()).map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect(),
            Value::String(x) => s.fallback = x.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect(),
            _ => {}
        }
        let cfg = app.cfg();
        if let Some(v) = [&s.model, &s.effort].into_iter().flatten().chain(&s.fallback).find(|v| !valid(v)) {
            return Err(format!("'{v}' is not a valid model, effort or fallback entry"));
        }
        if let Some(alias) = std::iter::once(&s.agent).filter(|_| role_name != "brain").chain(&s.fallback).map(|e| e.split(':').next().unwrap_or("")).find(|a| !cfg.agents.contains_key(*a)) {
            return Err(format!("no agent alias '{alias}' in config.toml"));
        }
        app.set_kv(&format!("models:{role_name}"), Some(&serde_json::to_string(&s).unwrap_or_default()));
    } else {
        app.set_kv(&format!("models:{role_name}"), None);
    }
    let (now, source) = role_with_source(app, role_name);
    app.event("", "", "models", role_name, &format!("{role_name}: {} ({source})", line(&now)), "");
    app.emit("models", state(app));
    if role(app, "manager") != before {
        crate::manager::restart();
    }
    Ok(now)
}

fn line(s: &RoleSetting) -> String {
    let mut out = format!("agent={} model={} effort={}", s.agent, s.model.as_deref().unwrap_or("-"), s.effort.as_deref().unwrap_or("-"));
    if !s.fallback.is_empty() {
        out += &format!(" fallback={}", s.fallback.join(","));
    }
    out
}

/// GET /api/models: every role's effective setting with its source, and the choices the page offers.
pub fn state(app: &Shared) -> Value {
    let cfg = app.cfg();
    let roles: serde_json::Map<String, Value> = ROLES
        .iter()
        .map(|r| {
            let (s, source) = role_with_source(app, r);
            let mut v = json!(s);
            v["source"] = json!(source);
            (r.to_string(), v)
        })
        .collect();
    json!({"roles": roles, "choices": {"agents": cfg.agents.keys().collect::<Vec<_>>(), "models": get::<Vec<String>>(&cfg.raw, &["models", "choices"]).unwrap_or_default(),
        "efforts": get::<Vec<String>>(&cfg.raw, &["models", "efforts"]).unwrap_or_else(|| EFFORTS.map(String::from).to_vec())}})
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/models", GET(|State(app): State<Shared>| async move { Json(state(&app)) })).route(
        "/api/models/{role}",
        axum::routing::post(|State(app): State<Shared>, UrlPath(r): UrlPath<String>, Json(b): Json<Value>| async move { save(&app, &r, Some(&b)).map(|s| Json(json!(s))).map_err(bad) })
            .delete(|State(app): State<Shared>, UrlPath(r): UrlPath<String>| async move { save(&app, &r, None).map(|s| Json(json!(s))).map_err(bad) }),
    )
}

/// `tirith models`, `tirith models set <role> [--agent A] [--model M] [--effort E] [--fallback "a:m,b"]`,
/// `tirith models reset <role>`; `None` when `pos` is not one of this module's commands.
pub fn cli(pos: &[&str], flag: Flag) -> Option<Result<(), String>> {
    let show = |body: Result<String, String>| -> Result<(), String> {
        let s: RoleSetting = serde_json::from_str(&body?).map_err(|e| e.to_string())?;
        println!("{}", line(&s));
        Ok(())
    };
    Some(match pos {
        ["models"] => call("GET", "/api/models", None).and_then(|b| {
            let v: Value = serde_json::from_str(&b).map_err(|e| e.to_string())?;
            for r in ROLES {
                let s: RoleSetting = serde_json::from_value(v["roles"][r].clone()).unwrap_or_default();
                println!("{r:<8} {}  [{}]", line(&s), v["roles"][r]["source"].as_str().unwrap_or(""));
            }
            Ok(())
        }),
        ["models", "set", r] => {
            let body: serde_json::Map<String, Value> = ["agent", "model", "effort", "fallback"].iter().filter_map(|k| flag(&format!("--{k}")).map(|v| (k.to_string(), json!(v)))).collect();
            show(call("POST", &format!("/api/models/{r}"), Some(Value::Object(body))))
        }
        ["models", "reset", r] => show(call("DELETE", &format!("/api/models/{r}"), None)),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn codex_lane_runs_commands() {
        let c = crate::config::Config::default();
        let (_, _, resume, args) = super::flags(&c, "codex");
        for a in [resume, args] {
            assert!(a.iter().any(|x| x == super::NO_SANDBOX));
            assert!(!a.iter().any(|x| x == "--sandbox"));
        }
    }
}
