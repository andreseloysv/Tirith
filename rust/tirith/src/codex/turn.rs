//! One exec turn: set it up, start the agent, read its answer within the turn's time, and clean up.
use super::{command::command_line, reading::Reading, setup::Setup};
use crate::{
    app::{kill_group, Shared},
    manager, models,
};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};

pub async fn turn(
    app: &Shared,
    role: &str,
    project: &str,
    resume: Option<&str>,
    prompt: (&str, &str),
    mut event: impl FnMut(&Value),
    started: impl FnOnce(u32),
) -> Result<String, String> {
    let setting = models::role(app, role); // a bot's conversation passes its own role (its key) here
    let setup = Setup::new(app, role, project, &setting.agent)?;
    let model = (setting.model.as_deref(), setting.effort.as_deref());
    let line = command_line(app, &setup, model, resume, prompt)?;
    let (mut child, stdin, stdout) =
        manager::spawn_chat(app, &setup.who, project, line.argv.clone())?;
    let pid = child.id().unwrap_or(0);
    started(pid);
    let input = line.stdin.as_deref();
    let work = read(&mut child, stdin, stdout, input, &mut event, &setup);
    let result = tokio::time::timeout(setup.limit(), work).await;
    kill_group(pid, libc::SIGKILL);
    let _ = child.wait().await;
    // Copilot writes what the turn used into a file as it ends.
    let used = line.usage.as_deref().and_then(|f| std::fs::read(f).ok());
    if let Some(data) = used.and_then(|b| serde_json::from_slice::<Value>(&b).ok()) {
        event(&json!({"type": "copilot.usage", "data": data}));
    }
    drop(line.scratch);
    result.map_err(|_| format!("{} timed out", setup.agent.name))?
}

/// Writes the message to stdin (when the agent takes it there), reads every event, and waits for the process.
async fn read(
    child: &mut Child,
    mut stdin: ChildStdin,
    stdout: ChildStdout,
    input: Option<&str>,
    event: &mut impl FnMut(&Value),
    setup: &Setup,
) -> Result<String, String> {
    if let Some(input) = input {
        let wrote = stdin.write_all(input.as_bytes()).await;
        wrote.map_err(|e| e.to_string())?
    }
    drop(stdin);
    let mut lines = BufReader::new(stdout).lines();
    let mut reading = Reading::default();
    while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        event(&msg);
        reading.take(&msg);
    }
    let status = child.wait().await.map_err(|e| e.to_string())?;
    reading.answer(status, setup.agent.name, &setup.who)
}
