//! `tirith`: the daemon (`tirith serve`) and its thin command-line client. Every other command is one HTTP
//! call to the daemon on 127.0.0.1, so it starts in milliseconds.

mod app;
mod attach;
mod brain;
mod client;
mod config;
mod guard;
mod lanes;
mod manager;
mod merge;
mod metrics;
mod models;
mod plan;
mod rooms;
mod rules;
mod sandbox;
mod server;

use client::{call, call_with, print};
use serde_json::{json, Value};
use std::io::Read;
use std::time::Duration;

const USAGE: &str = "tirith serve                                   run the daemon (page on http://127.0.0.1:<port>/)
tirith status                                  lanes, load and the merge queue
tirith plan show <project>                     the plan with what works on each item
tirith plan add <project> <step> \"<text>\"      add an item to a step
tirith lane start <project> <step> \"<item>\" [--agent <alias>]
tirith lane stop <lane>
tirith lane log <lane>
tirith ask \"<question>\"                       (inside a lane) ask the manager, then end your turn
tirith answer <lane> \"<answer>\"                (the manager) answer a waiting lane
tirith ask-owner --kind credentials|payment|account|device|decision \"<question>\"   (the manager)
tirith autopilot on|off <project>              let Tirith start the plan's items by itself, or not
tirith say \"<text>\"                            write to the manager
tirith hook <Event>                            agent hook bridge (stdin JSON; also --agent A --event E)
tirith hooks install                           point the agents' hooks at the daemon over HTTP
(modules add: tirith room …, tirith net …, tirith models …, tirith guard …)";

type ModuleCli = fn(&[&str], client::Flag) -> Option<Result<(), String>>;
const MODULE_CLI: [ModuleCli; 4] = [rooms::cli, sandbox::cli, models::cli, guard::cli];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("serve") {
        // A CLI command piped into `head` ends quietly instead of panicking on the closed pipe.
        unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) };
    }
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let mut pos: Vec<&str> = vec![];
    let mut skip = false;
    for a in &args {
        if skip {
            skip = false;
        } else if a.starts_with("--") {
            skip = true;
        } else {
            pos.push(a);
        }
    }
    let result = MODULE_CLI.iter().find_map(|f| f(&pos, &flag)).unwrap_or_else(|| match pos.as_slice() {
        ["serve"] => serve(),
        ["hook", rest @ ..] => hook(flag("--event").or(rest.first().map(|s| s.to_string())).unwrap_or_default()),
        ["hooks", "install"] => install_hooks(),
        ["status"] => status(),
        ["plan", "show", project] => plan_show(project),
        ["plan", "add", project, step, text] => print(call("POST", &format!("/api/plan/{project}"), Some(json!({"step": step, "text": text})))),
        ["lane", "start", project, step, item] => print(call("POST", "/api/lanes", Some(json!({"project": project, "step": step, "item": item, "agent": flag("--agent")})))),
        ["lane", "stop", id] => print(call("POST", &format!("/api/lanes/{id}/stop"), None)),
        ["lane", "log", id] => call("GET", &format!("/api/lanes/{id}/log"), None).map(|b| println!("{b}")),
        ["ask", question] => {
            let lane = std::env::var("TIRITH_LANE").map_err(|_| "tirith ask works inside a lane (TIRITH_LANE); the manager asks the owner with tirith ask-owner".to_string());
            lane.and_then(|l| call("POST", &format!("/api/lanes/{l}/ask"), Some(json!({"question": question}))))
                .map(|_| println!("Asked the manager. End your turn now; you will be resumed with the answer."))
        }
        ["answer", id, answer] => print(call("POST", &format!("/api/lanes/{id}/answer"), Some(json!({"answer": answer})))),
        ["ask-owner", question] => {
            if std::env::var("TIRITH_LANE").is_ok_and(|l| !l.is_empty()) {
                Err("a lane does not ask the owner: ask the manager with tirith ask \"<question>\"".to_string())
            } else {
                let body = json!({"kind": flag("--kind").unwrap_or_default(), "question": question});
                call("POST", "/api/owner", Some(body)).map(|b| println!("{b}"))
            }
        }
        ["autopilot", on @ ("on" | "off"), project] => print(call("POST", &format!("/api/autopilot/{project}"), Some(json!({"on": *on == "on"})))),
        ["say", text] => print(call("POST", "/api/chat", Some(json!({"text": text})))),
        _ => Err(USAGE.to_string()),
    });
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn status() -> Result<(), String> {
    let s: Value = serde_json::from_str(&call("GET", "/api/state", None)?).map_err(|e| e.to_string())?;
    let lanes = s["lanes"].as_array().cloned().unwrap_or_default();
    let mut shown = 0;
    for l in lanes.iter().filter(|l| matches!(l["status"].as_str(), Some("queued" | "running" | "waiting" | "merge-queued" | "merging"))) {
        println!(
            "{:<14} {:<40} {}:{} {}",
            l["status"].as_str().unwrap_or(""),
            l["id"].as_str().unwrap_or(""),
            l["project"].as_str().unwrap_or(""),
            l["step"].as_str().unwrap_or(""),
            l["item"].as_str().unwrap_or("")
        );
        shown += 1;
    }
    if shown == 0 {
        println!("no lane is working");
    }
    let mut ended: Vec<&Value> = lanes.iter().filter(|l| l["ended_at"].is_string() && !matches!(l["status"].as_str(), Some("merge-queued" | "merging"))).collect();
    ended.sort_by_key(|l| l["ended_at"].as_str().unwrap_or("").to_string());
    for l in ended.iter().rev().take(8) {
        println!("{:<14} {:<40} {}", l["status"].as_str().unwrap_or(""), l["id"].as_str().unwrap_or(""), l["note"].as_str().unwrap_or(""));
    }
    Ok(())
}

fn plan_show(project: &str) -> Result<(), String> {
    let p: Value = serde_json::from_str(&call("GET", &format!("/api/plan/{project}"), None)?).map_err(|e| e.to_string())?;
    for st in p["steps"].as_array().into_iter().flatten() {
        let items = st["items"].as_array().cloned().unwrap_or_default();
        let open: Vec<&Value> = items.iter().filter(|i| i["done"] != true).collect();
        println!("## {}. {}  ({} open, {} done)", st["num"].as_str().unwrap_or(""), st["title"].as_str().unwrap_or(""), open.len(), items.len() - open.len());
        for it in open {
            let lane = it["lane"].as_object().map(|l| format!("  [{} {}]", l["status"].as_str().unwrap_or(""), l["id"].as_str().unwrap_or(""))).unwrap_or_default();
            println!("  - [ ] {}{lane}", it["text"].as_str().unwrap_or(""));
        }
    }
    Ok(())
}

/// The command-hook bridge for agents without HTTP hooks: never breaks the agent (any failure lets the call
/// through); a refusal is exit 2 with the reason on stderr, which every supported agent understands.
fn hook(event: String) -> Result<(), String> {
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);
    let headers = [("X-Tirith-Lane", std::env::var("TIRITH_LANE").unwrap_or_default()), ("X-Tirith-Role", std::env::var("TIRITH_ROLE").unwrap_or_default())];
    let Ok(body) = call_with("POST", &format!("/hook/{event}"), input, &headers, Duration::from_secs(5)) else { return Ok(()) };
    let v: Value = serde_json::from_str(&body).unwrap_or(json!({}));
    let deny = v["hookSpecificOutput"]["permissionDecisionReason"].as_str().filter(|_| v["hookSpecificOutput"]["permissionDecision"] == "deny");
    let block = v["reason"].as_str().filter(|_| v["decision"] == "block");
    if let Some(why) = deny.or(block) {
        eprintln!("{why}");
        std::process::exit(2);
    }
    if v["hookSpecificOutput"].is_object() {
        println!("{body}"); // an allow decision or context for the agent
    }
    Ok(())
}

/// Rewrites the hooks of the configured agents whose settings file uses `tirith hook …` into HTTP hooks on the
/// daemon (a backup of each file is kept next to it). Agents without HTTP hooks keep the command bridge.
fn install_hooks() -> Result<(), String> {
    let cfg = config::Config::load();
    let agent = cfg.agents.get("anthropic").ok_or("no [agents.anthropic] in config.toml")?;
    let path = config::expand(agent.settings.as_deref().ok_or("[agents.anthropic] has no settings path")?);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| "{}".into());
    std::fs::write(path.with_extension("json.before-tirith-http"), &text).map_err(|e| e.to_string())?;
    let mut settings: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    for event in ["PreToolUse", "PostToolUse", "UserPromptSubmit", "Stop"] {
        let ours = json!({"type": "http", "url": format!("http://127.0.0.1:{}/hook/{event}", cfg.port),
            "headers": {"X-Tirith-Lane": "$TIRITH_LANE", "X-Tirith-Role": "$TIRITH_ROLE"},
            "allowedEnvVars": ["TIRITH_LANE", "TIRITH_ROLE"], "timeout": 5});
        let groups = settings["hooks"][event].as_array().cloned().unwrap_or_default();
        let mut kept: Vec<Value> = groups
            .into_iter()
            .map(|mut g| {
                if let Some(hs) = g["hooks"].as_array() {
                    let hs: Vec<Value> =
                        hs.iter().filter(|h| !h["command"].as_str().unwrap_or("").starts_with("tirith hook") && !h["url"].as_str().unwrap_or("").contains("/hook/")).cloned().collect();
                    g["hooks"] = json!(hs);
                }
                g
            })
            .filter(|g| g["hooks"].as_array().is_some_and(|h| !h.is_empty()))
            .collect();
        kept.push(json!({"hooks": [ours]}));
        settings["hooks"][event] = json!(kept);
    }
    let out = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, out + "\n").map_err(|e| e.to_string())?;
    println!("{}: hooks now call http://127.0.0.1:{}/hook/<event>", path.display(), cfg.port);
    Ok(())
}

fn serve() -> Result<(), String> {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let app = app::App::open();
        let port = app.cfg().port;
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await.map_err(|e| format!("port {port} is taken (is tirith already running?): {e}"))?;
        for start in [sandbox::start, rooms::start, brain::start, guard::start, metrics::start] {
            start(app.clone());
        }
        tokio::spawn(lanes::scheduler(app.clone()));
        tokio::spawn(merge::worker(app.clone()));
        tokio::spawn(manager::run(app.clone()));
        let flusher = app.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(flusher.cfg().plan_commit_s)).await;
                plan::flush(&flusher, None).await;
                flusher.trim();
            }
        });
        println!("tirith: http://127.0.0.1:{port}/ (home {})", app.home.display());
        axum::serve(listener, server::router(app, port)).await.map_err(|e| e.to_string())
    })
}
