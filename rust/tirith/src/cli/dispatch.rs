//! The command the positional words name: a module's own first, then the daemon's.
use super::{ask, hook::hook, hooks::install_hooks, open, plan, serve::serve, status::status};
use crate::client::{call, print, Flag};
use crate::{bots, chat, guard, models, projects, rooms, sandbox};
use serde_json::json;

const USAGE: &str = include_str!("../../assets/usage.txt");

type ModuleCli = fn(&[&str], Flag) -> Option<Result<(), String>>;
const MODULE_CLI: [ModuleCli; 12] = [
    crate::gateway::cli,
    crate::oauth::cli,
    rooms::cli,
    sandbox::cli,
    models::cli,
    guard::cli,
    projects::cli,
    bots::cli,
    chat::cli,
    crate::turns::cli,
    crate::teach::cli,
    crate::lab::cli,
];

pub(super) fn run(pos: &[&str], flag: Flag, project: &str) -> Result<(), String> {
    MODULE_CLI
        .iter()
        .find_map(|f| f(pos, flag))
        .unwrap_or_else(|| daemon(pos, flag, project))
}

/// The commands of the daemon itself.
fn daemon(pos: &[&str], flag: Flag, project: &str) -> Result<(), String> {
    match pos {
        ["serve"] => serve(),
        ["hook", rest @ ..] => hook(
            flag("--event")
                .or(rest.first().map(|s| s.to_string()))
                .unwrap_or_default(),
            flag("--home"),
            flag("--agent").unwrap_or_default(),
        ),
        ["hooks", "install"] => install_hooks(),
        ["hooks", "remove"] => crate::setup::unhook::cli(),
        ["open"] => open::open(),
        ["open", "link"] => open::print(),
        ["status"] => status(project),
        ["plan", "show", project] => plan::plan_show(project),
        ["plan", "step", "delete", project, num] => print(call(
            "POST",
            &format!("/api/plan/{project}/steps/{num}/delete"),
            None,
        )),
        ["plan", "step", project, num, title] => plan::plan_step(project, num, title, ""),
        ["plan", "step", project, num, title, intro] => plan::plan_step(project, num, title, intro),
        ["plan", "add", project, step, text] => print(call(
            "POST",
            &format!("/api/plan/{project}"),
            Some(json!({"step": step, "text": text})),
        )),
        ["lane", "start", project, step, item] => print(call(
            "POST",
            "/api/lanes",
            Some(json!({"project": project, "step": step, "item": item, "agent": flag("--agent")})),
        )),
        ["lane", "stop", id] => print(call("POST", &format!("/api/lanes/{id}/stop"), None)),
        ["lane", "delete", id] => print(call("POST", &format!("/api/lanes/{id}/delete"), None)),
        ["card", "close", id, why] => print(call(
            "POST",
            &format!("/api/owner/{id}/close"),
            Some(json!({"why": why, "by": "manager"})),
        )),
        ["lane", "log", id] => {
            call("GET", &format!("/api/lanes/{id}/log"), None).map(|b| println!("{b}"))
        }
        ["ask", question] => ask::ask(question),
        ["answer", id, answer] => ask::answer(id, answer),
        ["ask-owner", question] => ask::ask_owner(question, project, flag),
        ["autopilot", on @ ("on" | "off"), project] => print(call(
            "POST",
            &format!("/api/autopilot/{project}"),
            Some(json!({"on": *on == "on"})),
        )),
        ["say", text] => print(call(
            "POST",
            "/api/chat",
            Some(json!({"text": text, "project": project})),
        )),
        _ => Err(USAGE.to_string()),
    }
}
