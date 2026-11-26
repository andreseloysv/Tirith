//! Starting a streaming chat agent, and the answers to its permission requests.
use crate::app::Shared;
use crate::sandbox;
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::process::{Child, ChildStdin, ChildStdout};
/// Starts a streaming chat agent on behalf of `role` in `<home>/<role>` (a project's chat in its own folder below it, so the agent's own notes stay with the project), sandboxed, logging to `logs/<role>.log`. A bot's own turn role (the CEO, the judge) runs in its bot folder `<home>/manager/<role>`, like its conversation does.
pub fn spawn_chat(
    app: &Shared,
    role: &str,
    project: &str,
    argv: Vec<String>,
) -> Result<(Child, ChildStdin, ChildStdout), String> {
    let quick = super::quick_shell::env(app, project, argv.first().map_or("", |p| p)); // a bot's shell starts without the owner's profile
    let launch = sandbox::wrap(app, &format!("{role}@{project}"), argv)?;
    let dir = if app.cfg().factory.role::<String>(role, "scope").as_deref() == Some("own") {
        app.home.join("manager").join(role)
    } else {
        app.home.join(role).join(project)
    };
    let dir = crate::lab::seat(dir); // a B run of a live experiment runs in its branch's worktree
    let _ = std::fs::create_dir_all(&dir);
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(app.home.join(format!("logs/{role}.log")))
        .map_err(|e| e.to_string())?;
    let mut child = launch
        .command(app)
        .current_dir(&dir)
        .env("TIRITH_ROLE", role)
        .env("TIRITH_PROJECT", project)
        .env(
            "CARGO_TARGET_DIR",
            crate::lanes::shared_target(app, if project.is_empty() { role } else { project }),
        ) // managers, bots and the CEO build into the shared cache: a cargo of theirs in a project folder leaves no target/ there
        .env("TIRITH_HOME", &app.home)
        .envs(quick)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(log)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("cannot start the {role} ({}): {e}", launch.program))?;
    let (stdin, stdout) = (
        child.stdin.take().ok_or("no stdin")?,
        child.stdout.take().ok_or("no stdout")?,
    );
    Ok((child, stdin, stdout))
}
pub(super) fn control_reply(msg: &Value, behavior: Value) -> String {
    json!({"type": "control_response", "response": {"subtype": "success", "request_id": msg["request_id"], "response": behavior}}).to_string()
}
/// A tool call of a turn: with `access = "read"` the file-writing tools are denied and everything else (including every `tirith` command that only reads) is allowed; any other access allows all.
pub(super) fn turn_permission(read_only: bool, tool: &str, input: &Value) -> Value {
    if read_only && crate::rules::EDIT_TOOLS.contains(&tool) {
        json!({"behavior": "deny", "message": "this role is read-only (access = \"read\")"})
    } else {
        json!({"behavior": "allow", "updatedInput": input})
    }
}
