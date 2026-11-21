//! Models per factory role and the fallback chain: the owner's override from the page, else `[roles.<role>]` of config.toml laid over the factory's role, else that role as the factory defines it.
use crate::app::{bad, Shared};
use crate::client::{call, Flag};
use crate::config::{get, Config};
use axum::extract::{Path as UrlPath, State};
use axum::{routing::get as GET, Json, Router};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{path::Path, sync::OnceLock};
const EFFORTS: [&str; 6] = ["low", "medium", "high", "xhigh", "max", "ultra"];
/// The factory role that runs lanes (its `session = "lane"` one), by name.
pub fn lane_role(cfg: &Config) -> String {
    cfg.factory.find_role("lane", None).unwrap_or_else(|| "worker".into())
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
/// An agent command line: the alias it belongs to and the full argv (program first).
pub struct AgentCmd {
    pub alias: String,
    pub argv: Vec<String>,
}
/// The effective setting of a role and where it comes from: "page", "config" (config.toml's `[roles.<role>]` sets its agent, model, effort or fallback) or "default" (the factory's role alone, its `extends` resolved).
pub fn role_with_source(app: &Shared, role: &str) -> (RoleSetting, &'static str) {
    if let Some(s) = app.kv(&format!("models:{role}")).and_then(|j| serde_json::from_str(&j).ok()) {
        return (s, "page");
    }
    let cfg = app.cfg();
    if role.starts_with("bot-") {
        let chat = cfg.factory.find_role("chat", Some("project")).unwrap_or_else(|| "manager".into());
        return (self::role(app, &chat), "managers"); // one of the owner's bots without a model of its own has the project chat role's (a system bot keeps its factory role's below)
    }
    let f = &cfg.factory;
    let from_config = get::<toml::Table>(&cfg.raw, &["roles", role]).is_some_and(|t| ["agent", "model", "effort", "fallback"].iter().any(|k| t.get(*k).is_some_and(|v| v.as_str().is_none_or(|s| !s.is_empty()))));
    let empty = |v: Option<String>| v.filter(|s| !s.is_empty());
    let agent = empty(f.role(role, "agent")).or_else(|| empty(f.role(&lane_role(&cfg), "agent"))).unwrap_or_else(|| "anthropic".into());
    let s = RoleSetting { agent, model: empty(f.role(role, "model")), effort: empty(f.role(role, "effort")), fallback: f.role(role, "fallback").unwrap_or_default() };
    (s, if from_config { "config" } else { "default" })
}
pub fn role(app: &Shared, role: &str) -> RoleSetting {
    role_with_source(app, role).0
}
/// Per alias: (model flag, effort flag, resume args, lane args) from the factory's `[agents.<alias>]`, with config.toml laid over it; an empty flag is no flag.
fn flags(cfg: &Config, alias: &str) -> (Option<String>, Option<String>, Vec<String>, Vec<String>) {
    let key = |k: &str| cfg.factory.agent::<String>(alias, k).filter(|s| !s.is_empty());
    (key("model_flag"), key("effort_flag"), cfg.factory.agent(alias, "resume_args").unwrap_or_default(), cfg.factory.agent::<Vec<String>>(alias, "args").or_else(|| cfg.factory.agent(alias, "lane_args")).unwrap_or_default())
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
/// The browser MCP (Playwright, `tools/browser/launch.mjs` of this repo; its node_modules stay in `<home>/browser-mcp`) every agent run carries in its own dialect, so no agent's settings file is touched: Muse one `-c mcpServers.tirith_browser=…` at the argv's end (never the whole map; its env takes only the whole-value `${TIRITH_HOME}` both spawns set), Codex and anthropic right after the program (Codex `-c mcp_servers.tirith_browser={…}` plus desktop-only `node_repl` off, no `~/.codex` profile; anthropic `--mcp-config=`/`--allowedTools=` for `mcp__tirith_browser__*`, `=` so its variadic flags swallow nothing later). Lanes always get `--isolated` (several at once, nothing left behind, never the owner's browser); managers, bots and the CEO get Tirith's persistent profile (`<home>/browser/profile`, the launcher's default), `--isolated` instead while another live session holds that profile, or `--extension` (the owner's own Edge) with `[browser] use = "mine"` in config.toml. No node or no launcher: no browser, and the run goes on without one. Agents spawn MCP servers with a clean env (only the variables named in this config), so inside Tirith's sandbox the server env also names `TIRITH_SANDBOXED` (1) and the four proxy variables, forwarded by name and resolved by each agent from its own env, where `wrap` put them (Muse and anthropic as a whole-value `${VAR}`, Codex through `env_vars`): without them the launcher never learns it runs sandboxed (Chromium starts its own sandbox, macOS refuses to nest it — "sandbox init: Operation not permitted" — and Edge dies) nor which proxy to leave through. Muse accepts no literal env value, so its flag rides as `${TIRITH_SANDBOXED}`; the launcher still sees plain "1".
fn browser_server(app: &Shared, lane: bool) -> Option<(String, Vec<String>)> {
    let launch = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/browser/launch.mjs"));
    let node = launch.is_file().then(|| node_bin(app)).flatten()?;
    let busy = std::fs::read_link(app.home.join("browser/profile/SingletonLock")).unwrap_or_default().to_string_lossy().rsplit('-').next().and_then(|p| p.parse().ok()).is_some_and(crate::app::alive);
    let mode = (!lane && get::<String>(&app.cfg().raw, &["browser", "use"]).as_deref() == Some("mine")).then_some("--extension").or((lane || busy).then_some("--isolated"));
    Some((node, [launch.display().to_string()].into_iter().chain(mode.map(String::from)).collect()))
}
pub fn browser_args_for(app: &Shared, alias: &str, lane: bool) -> Vec<String> {
    let Some((node, args)) = browser_server(app, lane) else { return Vec::new() };
    let (home, t) = (app.home.display().to_string(), |s: &str| toml::Value::String(s.into()).to_string());
    let proxy = ["HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY", "NO_PROXY"];
    let boxed = crate::sandbox::sandboxed(app);
    let env = |home: &str, sandboxed: &str| -> Value {
        let extra = if boxed { proxy.iter().map(|k| (k.to_string(), json!(format!("${{{k}}}")))).chain([("TIRITH_SANDBOXED".to_string(), json!(sandboxed))]).collect::<Vec<_>>() } else { Vec::new() };
        Value::Object([("TIRITH_HOME".to_string(), json!(home))].into_iter().chain(extra).collect())
    };
    let list = args.iter().map(|a| t(a)).collect::<Vec<_>>().join(", ");
    let named = if boxed { format!(", env_vars = [{}]", proxy.iter().map(|k| t(k)).collect::<Vec<_>>().join(", ")) } else { String::new() };
    let flag = if boxed { ", TIRITH_SANDBOXED = \"1\"" } else { "" };
    match alias {
        "muse" => vec!["-c".into(), format!("mcpServers.tirith_browser={}", json!({"command": node, "args": args, "env": env("${TIRITH_HOME}", "${TIRITH_SANDBOXED}")}))],
        "anthropic" => vec![format!("--mcp-config={}", json!({"mcpServers": {"tirith_browser": {"command": node, "args": args, "env": env(&home, "1")}}})), "--allowedTools=mcp__tirith_browser__*".into()],
        "codex" => Vec::from([
            "-c".to_string(),
            format!("mcp_servers.tirith_browser={{command = {}, args = [{}], startup_timeout_sec = 30, tool_timeout_sec = 90, env = {{TIRITH_HOME = {}{}}}{}}}", t(&node), list, t(&home), flag, named),
            "-c".into(),
            "mcp_servers.node_repl.enabled=false".into(),
        ]),
        _ => Vec::new(),
    }
}
fn add_browser(app: &Shared, argv: &mut Vec<String>, alias: &str, lane: bool) {
    let at = if alias == "muse" { argv.len() } else { 1 }; // Muse's -c only works after its subcommand, the others' flags belong right after the program
    drop(argv.splice(at..at, browser_args_for(app, alias, lane)));
}
/// Node for the browser launcher: on the children's PATH, else the newest `~/.nvm` install (the daemon's own PATH may have neither).
fn node_bin(app: &Shared) -> Option<String> {
    let on_path = app.child_path().split(':').map(|d| Path::new(d).join("node")).find(|p| p.is_file());
    on_path.or_else(|| std::fs::read_dir(crate::config::home_dir().join(".nvm/versions/node")).ok().into_iter().flatten().flatten().map(|e| e.path().join("bin/node")).filter(|p| p.is_file()).max()).map(|p| p.display().to_string())
}
/// The argv of a lane run. `agent`/`model` override the lane role (a fallback relaunch passes them). `resume = Some((session_id, message))` continues that agent session with `message` instead of the brief; an alias without `resume_args` starts fresh with the brief.
pub fn lane_argv(app: &Shared, agent: Option<&str>, model: Option<&str>, brief: &Path, workspace: &Path, resume: Option<(&str, &str)>) -> Result<AgentCmd, String> {
    let cfg = app.cfg();
    let lane = lane_role(&cfg);
    let w = role(app, &lane);
    let alias = agent.unwrap_or(&w.agent).to_string();
    let command = cfg.factory.command(&alias).ok_or(format!("no agent alias '{alias}' in config.toml"))?;
    let same = alias == w.agent;
    let model = model.map(String::from).or(if same { w.model } else { None });
    let effort = if same { w.effort } else { None };
    let (mflag, eflag, resume_args, args) = flags(&cfg, &alias);
    let (brief, ws, bin) = (brief.display().to_string(), workspace.display().to_string(), command.rsplit('/').next().unwrap_or(&command).to_string());
    let (session, message) = resume.unwrap_or_default();
    let fill = |s: &String| s.replace("{brief}", &brief).replace("{workspace}", &ws).replace("{session}", session).replace("{message}", message);
    let mut argv: Vec<String> = if resume.is_some() && !resume_args.is_empty() {
        std::iter::once(command).chain(resume_args.iter().map(fill)).collect()
    } else if Some(alias.as_str()) == get::<String>(&cfg.raw, &["roles", &lane, "agent"]).or_else(|| cfg.factory.role(&lane, "agent")).as_deref() && !cfg.launch.is_empty() {
        cfg.launch.iter().map(fill).collect()
    } else {
        std::iter::once(command).chain(args.iter().map(fill)).collect()
    };
    set_flag(&mut argv, &mflag, &["--model", "-m"], &model);
    set_flag(&mut argv, &eflag, &["--effort", "--reasoning-effort"], &effort);
    let _ = (alias == "codex").then(|| crate::codex::options(&cfg, &mut argv, effort.as_deref()));
    let _ = argv.first().is_some_and(|a| a.rsplit('/').next() == Some(bin.as_str())).then(|| add_browser(app, &mut argv, &alias, true)); // a custom [launch] line naming another binary than the alias's command never gets its agent's browser flags
    Ok(AgentCmd { alias, argv })
}
/// The argv of a streaming chat session (stream-json in and out) for a role. `resume` continues that session id. The caller appends its own system prompt.
pub fn chat_argv(app: &Shared, role_name: &str, resume: Option<&str>) -> Result<AgentCmd, String> {
    let cfg = app.cfg();
    let r = role(app, role_name);
    let mut argv = vec![cfg.factory.command(&r.agent).ok_or(format!("no agent alias '{}' in config.toml", r.agent))?];
    // Exec agents do not speak this chat protocol: their chat and turn roles run as exec turns. Never hand them the default flags of the other one.
    let Some(chat) = cfg.factory.agent::<Vec<String>>(&r.agent, "chat_args") else {
        return Err(format!("agent '{}' has no stream-json chat mode; set chat_args for it in config.toml or run it as exec turns", r.agent));
    };
    argv.extend(chat);
    let (mflag, eflag, _, _) = flags(&cfg, &r.agent);
    set_flag(&mut argv, &mflag, &["--model", "-m"], &r.model);
    set_flag(&mut argv, &eflag, &["--effort", "--reasoning-effort"], &r.effort);
    argv.extend(resume.into_iter().flat_map(|id| ["--resume".to_string(), id.to_string()]));
    add_browser(app, &mut argv, &r.agent, false);
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
/// The next choice of a role's chain (its own agent, then `fallback`) that is not in `tried`: (alias, model). `tried` holds "alias" (every model of it is skipped) or "alias:model" (that one only).
pub fn fallback(app: &Shared, role_name: &str, tried: &[String]) -> Option<(String, Option<String>)> {
    let cfg = app.cfg();
    let r = role(app, role_name);
    let first = r.model.as_ref().map(|m| format!("{}:{m}", r.agent)).unwrap_or(r.agent.clone());
    std::iter::once(first).chain(r.fallback).find_map(|e| {
        let (alias, model) = e.split_once(':').map(|(a, m)| (a.to_string(), Some(m.to_string()))).unwrap_or((e.clone(), None));
        let used = tried.iter().any(|t| *t == e || *t == alias);
        (!used && cfg.factory.command(&alias).is_some()).then_some((alias, model))
    })
}
/// Saves the owner's setting of a role (fields absent from `b` keep their effective value, "" clears them), or with `None` goes back to config.toml. A chat role whose setting changed restarts its sessions (after their current turn).
pub fn save(app: &Shared, role_name: &str, b: Option<&Value>) -> Result<RoleSetting, String> {
    let roles = app.cfg().factory.roles();
    roles.iter().any(|r| r == role_name).then_some(()).ok_or_else(|| format!("role must be one of {}", roles.join(", ")))?; // the bots are factory roles too (scope = "own")
    let before = role(app, role_name);
    let saved = b.map(|b| updated(app, role_name, b)).transpose()?.map(|s| serde_json::to_string(&s).unwrap_or_default());
    app.set_kv(&format!("models:{role_name}"), saved.as_deref());
    let (now, source) = role_with_source(app, role_name);
    app.event("", "", "models", role_name, &format!("{role_name}: {} ({source})", line(&now)), "");
    app.emit("models", state(app));
    let chat = app.cfg().factory.role::<String>(role_name, "session").as_deref() == Some("chat");
    let _ = (now != before && (chat || app.cfg().factory.role::<String>(role_name, "scope").as_deref() == Some("own"))).then(crate::manager::restart);
    Ok(now)
}
fn updated(app: &Shared, role_name: &str, b: &Value) -> Result<RoleSetting, String> {
    let mut s = role(app, role_name);
    let field = |k: &str, cur: Option<String>| match &b[k] {
        Value::String(v) if v.trim().is_empty() => None,
        Value::String(v) => Some(v.trim().to_string()),
        _ => cur,
    };
    let agent = field("agent", Some(s.agent.clone())).unwrap_or_default();
    let _ = (agent != s.agent).then(|| (s.model, s.effort) = (None, None));
    s.agent = agent;
    s.model = field("model", s.model);
    s.effort = field("effort", s.effort);
    match &b["fallback"] {
        Value::Array(a) => s.fallback = a.iter().filter_map(|x| x.as_str()).map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect(),
        Value::String(x) => s.fallback = x.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect(),
        _ => {}
    }
    let cfg = app.cfg();
    [&s.model, &s.effort]
        .into_iter()
        .flatten()
        .chain(&s.fallback)
        .find(|v| !(v.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()) && v.chars().all(|c| c.is_ascii_alphanumeric() || "._:/+-[]".contains(c))))
        .map_or(Ok(()), |v| Err(format!("'{v}' is not a valid model, effort or fallback entry")))?;
    std::iter::once(&s.agent).chain(&s.fallback).map(|e| e.split(':').next().unwrap_or("")).find(|a| cfg.factory.command(a).is_none()).map_or(Ok(()), |alias| Err(format!("no agent alias '{alias}' in config.toml")))?;
    (s.agent == "codex")
        .then(crate::codex::catalog)
        .and_then(|c| {
            c.iter()
                .find(|m| Some(m["slug"].as_str().unwrap_or("")) == s.model.as_deref())
                .filter(|m| s.effort.as_ref().is_some_and(|e| !m["efforts"].as_array().is_some_and(|es| es.iter().any(|v| v == e))))
                .map(|m| format!("unsupported effort for {}: {}", s.model.as_deref().unwrap_or(""), m["efforts"]))
        })
        .map_or(Ok(()), Err)?;
    Ok(s)
}
fn line(s: &RoleSetting) -> String {
    format!("agent={} model={} effort={}{}", s.agent, s.model.as_deref().unwrap_or("-"), s.effort.as_deref().unwrap_or("-"), if s.fallback.is_empty() { String::new() } else { format!(" fallback={}", s.fallback.join(",")) })
}
/// Deterministic owner commands, available even when no model can answer.
pub fn command(app: &Shared, text: &str) -> Option<Result<String, String>> {
    let words: Vec<&str> = text.split_whitespace().collect();
    matches!(words.first(), Some(&"/model" | &"/models" | &"/modelo" | &"/modelos")).then_some(())?;
    let all_roles = app.cfg().factory.roles();
    let help = format!("Machine-wide models (all projects).\n/models — current settings and Codex models\n/model <{}|all> <agent>:<model> [effort]\n/model <role> reset — back to config.toml\nExample: /model manager codex (Codex's default model). Changes apply to the next turn or new lanes; running work finishes first.", all_roles.join("|"));
    Some((|| {
        if words.len() == 1 || words.get(1) == Some(&"help") {
            let roles = all_roles.iter().map(|r| format!("{r}: {}", line(&role(app, r)))).collect::<Vec<_>>().join("\n");
            let catalog = crate::codex::catalog()
                .iter()
                .map(|m| format!("{} ({})", m["slug"].as_str().unwrap_or(""), m["efforts"].as_array().into_iter().flatten().filter_map(Value::as_str).collect::<Vec<_>>().join(", ")))
                .collect::<Vec<_>>()
                .join("\n");
            return Ok(format!("{roles}\n\nCodex models:\n{catalog}\n\n{help}"));
        }
        ((3..=4).contains(&words.len())).then_some(()).ok_or_else(|| help.clone())?;
        let r = if words[1] == "todos" { "all" } else { words[1] };
        let roles: Vec<String> = match r {
            "all" => all_roles.clone(),
            _ if all_roles.iter().any(|x| x == r) => vec![r.to_string()],
            _ => return Err(help),
        };
        let reset = words[2] == "reset";
        (!reset || words.len() == 3).then_some(()).ok_or_else(|| help.clone())?;
        let mut body = json!({});
        if !reset {
            let (agent, model) = words[2].split_once(':').unwrap_or_else(|| if words[2].starts_with("gpt-") { ("codex", words[2]) } else { (words[2], "") });
            body = json!({"agent": agent, "model": model, "effort": words.get(3).copied().unwrap_or("")});
            roles.iter().try_for_each(|r| updated(app, r, &body).map(|_| ()))?; // validate every role before saving any of them
        }
        let lines: Vec<String> = roles.into_iter().map(|r| save(app, &r, (!reset).then_some(&body)).map(|s| format!("{r}: {}", line(&s)))).collect::<Result<_, _>>()?;
        Ok(format!("Saved for all projects:\n{}\nApplies to the next turn or new lanes; running work finishes first.", lines.join("\n")))
    })())
}
/// GET /api/models: every factory role's effective setting (the bots among them) with its source and its factory shape (session, scope, access, runs_on, name, face, show), the choices the page offers, and the closed list of events a bot's card offers its routes from (but `schedule`: its routines).
pub fn state(app: &Shared) -> Value {
    let cfg = app.cfg();
    let roles: serde_json::Map<String, Value> = cfg
        .factory
        .roles()
        .into_iter()
        .map(|r| {
            let (s, source) = role_with_source(app, &r);
            let mut v = json!(s);
            v["source"] = json!(source);
            ["session", "scope", "access", "runs_on", "name"].into_iter().filter_map(|k| cfg.factory.role::<String>(&r, k).map(|x| (k, x))).for_each(|(k, x)| v[k] = json!(x));
            v["show"] = json!(cfg.factory.role::<bool>(&r, "show"));
            v["face"] = json!({"shape": get::<String>(cfg.factory.table(), &["roles", &r, "face", "shape"]), "color": get::<String>(cfg.factory.table(), &["roles", &r, "face", "color"])});
            (r, v)
        })
        .collect();
    json!({"roles": roles, "events": crate::factory::EVENTS.split(';').filter(|e| *e != "schedule").collect::<Vec<_>>(), "choices": {"agents": cfg.factory.commands().iter().map(|(k, _)| k).collect::<Vec<_>>(), "codex": crate::codex::catalog(), "models": get::<Vec<String>>(&cfg.raw, &["models", "choices"]).unwrap_or_default(),
        "efforts": get::<Vec<String>>(&cfg.raw, &["models", "efforts"]).unwrap_or_else(|| EFFORTS.map(String::from).to_vec())}})
}
/// A paired phone's encrypted control request. Repeated save IDs cannot reapply old settings.
pub fn phone_response(app: &Shared, peer: &str, b: &Value) -> Value {
    let id = b["id"].as_str().unwrap_or("");
    let key = format!("models.rpc:{peer}:{id}");
    let result = if b["v"] != 1 || id.is_empty() || id.len() > 80 || !(id.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()) && id.chars().all(|c| c.is_ascii_alphanumeric() || "._:/+-[]".contains(c))) {
        Err("invalid model request".to_string())
    } else if let Some(cached) = app.kv(&key).and_then(|s| serde_json::from_str::<Result<(), String>>(&s).ok()) {
        cached
    } else {
        let role = b["role"].as_str().unwrap_or("");
        let result = match b["action"].as_str() {
            Some("get") => Ok(()),
            Some("set") if b["setting"].is_object() => save(app, role, Some(&b["setting"])).map(|_| ()),
            Some("reset") => save(app, role, None).map(|_| ()),
            _ => Err("unknown model action".to_string()),
        };
        let _ = (b["action"] != "get").then(|| app.set_kv(&key, Some(&json!(result).to_string())));
        result
    };
    json!({"v": 1, "id": id, "ok": result.is_ok(), "error": result.err(), "state": state(app)})
}
pub fn routes() -> Router<Shared> {
    Router::new().route("/api/models", GET(|State(app): State<Shared>| async move { Json(state(&app)) })).route(
        "/api/models/{role}",
        axum::routing::post(|State(app): State<Shared>, UrlPath(r): UrlPath<String>, Json(b): Json<Value>| async move { save(&app, &r, Some(&b)).map(|s| Json(json!(s))).map_err(bad) })
            .delete(|State(app): State<Shared>, UrlPath(r): UrlPath<String>| async move { save(&app, &r, None).map(|s| Json(json!(s))).map_err(bad) }),
    )
}
/// `tirith models`, `tirith models set <role> [--agent A] [--model M] [--effort E] [--fallback "a:m,b"]`, `tirith models reset <role>`; `None` when `pos` is not one of this module's commands.
pub fn cli(pos: &[&str], flag: Flag) -> Option<Result<(), String>> {
    let show = |body: Result<String, String>| serde_json::from_str::<RoleSetting>(&body?).map_err(|e| e.to_string()).map(|s| println!("{}", line(&s)));
    Some(match pos {
        ["models"] => call("GET", "/api/models", None)
            .and_then(|b| serde_json::from_str::<Value>(&b).map_err(|e| e.to_string()))
            .map(|v| v["roles"].as_object().into_iter().flatten().for_each(|(r, rv)| println!("{r:<10} {}  [{}]", line(&serde_json::from_value(rv.clone()).unwrap_or_default()), rv["source"].as_str().unwrap_or("")))),
        ["models", "set", r] => {
            let body: serde_json::Map<String, Value> = ["agent", "model", "effort", "fallback"].iter().filter_map(|k| flag(&format!("--{k}")).map(|v| (k.to_string(), json!(v)))).collect();
            show(call("POST", &format!("/api/models/{r}"), Some(Value::Object(body))))
        }
        ["models", "reset", r] => show(call("DELETE", &format!("/api/models/{r}"), None)),
        _ => return None,
    })
}
#[cfg(test)]
#[path = "../checks/models.rs"]
mod tests;
