//! Codex's local catalog and the JSONL exec turns of the agents without a streaming chat mode (Codex, Muse). Conversations resume their native sessions; text jobs run read-only.
use crate::{
    app::{kill_group, Shared},
    manager, models,
};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub fn catalog() -> Vec<Value> {
    let home = std::env::var_os("CODEX_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| crate::config::home_dir().join(".codex"));
    let data: Value = std::fs::read(home.join("models_cache.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    data["models"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|m| m["visibility"] == "list")
        .filter_map(|m| {
            Some(json!({"slug": m["slug"].as_str()?, "name": m["display_name"],
            "efforts": m["supported_reasoning_levels"].as_array().into_iter().flatten().filter_map(|e| e["effort"].as_str()).collect::<Vec<_>>()}))
        })
        .collect()
}

pub fn options(cfg: &crate::config::Config, argv: &mut Vec<String>, effort: Option<&str>) {
    if let Some(profile) = crate::config::get::<String>(&cfg.raw, &["agents", "codex", "profile"]) {
        argv.splice(1..1, ["--profile".into(), profile]);
    }
    if let Some(e) = effort {
        // Replace only this config key; unrelated -c overrides must survive.
        argv.retain(|a| !a.starts_with("--config=model_reasoning_effort="));
        for i in (1..argv.len()).rev() {
            if argv[i].starts_with("model_reasoning_effort=") && matches!(argv[i - 1].as_str(), "-c" | "--config") {
                argv.drain(i - 1..=i);
            }
        }
        argv.push(format!("--config=model_reasoning_effort={}", toml::Value::String(e.into())));
    }
}

/// The argv of one Muse turn. It runs unattended inside Tirith's own sandbox, so its approvals and inner sandbox are off (`--yolo`, the parallel of the Codex bypass above); a read-only role's file-writing tools are off too.
fn muse_argv(command: &str, read_only: bool, model: Option<&str>, effort: Option<&str>, workspace: &str, session: Option<&str>, prompt_file: &str) -> Vec<String> {
    let mut a: Vec<String> = [command, "exec", "--json", "--yolo", "--workspace", workspace].map(String::from).to_vec();
    a.extend(read_only.then(|| "--disable-write".to_string()));
    a.extend([("--model", model), ("--reasoning-effort", effort), ("--session-id", session), ("--prompt-file", Some(prompt_file))].into_iter().filter_map(|(f, v)| v.map(|v| [f.to_string(), v.to_string()])).flatten());
    a
}

pub async fn turn(app: &Shared, role: &str, project: &str, resume: Option<&str>, prompt: (&str, &str), mut event: impl FnMut(&Value), started: impl FnOnce(u32)) -> Result<String, String> {
    let (instructions, input) = prompt;
    let setting = models::role(app, role); // a bot's conversation passes its own role (its key) here
    let muse = setting.agent == "muse";
    let name = if muse { "the muse agent" } else { "Codex" };
    let cfg = app.cfg();
    // The role's factory shape, not its name, decides: `access = "read"` runs read-only; a chat role (a bot's key counts as one) keeps the chats' folder and the longer turn.
    let read_only = cfg.factory.role::<String>(role, "access").as_deref() == Some("read");
    let chat = cfg.factory.role::<String>(role, "session").is_none_or(|s| s == "chat");
    let who = if chat { "manager" } else { role };
    let command = cfg.factory.command(if muse { "muse" } else { "codex" }).ok_or(if muse { "no agent alias 'muse' in config.toml" } else { "Codex is not configured" })?;
    let dir = app.home.join(who).join(project);
    // Muse takes the message from a file, and a fresh session of it has never seen the role's instructions: they ride in its first message.
    let file = muse.then(|| dir.join(".turn-prompt.txt"));
    let argv = if let Some(file) = &file {
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(file, if resume.is_some() { input.to_string() } else { format!("{instructions}\n\n{input}") }).map_err(|e| e.to_string())?;
        muse_argv(&command, read_only, setting.model.as_deref(), setting.effort.as_deref(), &dir.display().to_string(), resume, &file.display().to_string())
    } else {
        let mut argv = vec![command, "exec".into(), "--json".into(), "--skip-git-repo-check".into()];
        argv.extend(if read_only { vec!["--sandbox".into(), "read-only".into(), "--ephemeral".into()] } else { vec!["--dangerously-bypass-approvals-and-sandbox".into()] });
        argv.extend(setting.model.into_iter().flat_map(|m| ["--model".to_string(), m]));
        options(&cfg, &mut argv, setting.effort.as_deref());
        argv.extend(["-c".into(), format!("developer_instructions={}", toml::Value::String(instructions.into()))]);
        argv.extend(resume.into_iter().flat_map(|id| ["resume".to_string(), id.to_string()]));
        argv.push("-".into());
        argv
    };
    let (mut child, mut stdin, stdout) = manager::spawn_chat(app, who, project, argv)?;
    let pid = child.id().unwrap_or(0);
    started(pid);
    let work = async {
        if !muse {
            stdin.write_all(input.as_bytes()).await.map_err(|e| e.to_string())?
        }
        drop(stdin);
        let mut lines = BufReader::new(stdout).lines();
        let (mut text, mut complete, mut failure) = (String::new(), false, None);
        while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
            let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
            event(&msg);
            // Each agent names its event kind its own way; the shapes do not collide.
            match msg["payload_type"].as_str().or_else(|| msg["type"].as_str()).unwrap_or("") {
                "item.completed" if msg["item"]["type"] == "agent_message" => {
                    text.push_str(msg["item"]["text"].as_str().unwrap_or(""));
                    text.push('\n');
                }
                "run.output.delta" => text.push_str(msg["payload"]["text"].as_str().unwrap_or("")),
                t if t.starts_with("run.terminal.") => {
                    complete = msg["payload"]["terminal"] == "completed";
                    if let Some(s) = msg["payload"]["text"].as_str().filter(|s| !s.is_empty()) {
                        text = s.to_string()
                    }
                    if !complete {
                        failure = Some(msg["payload"]["reason"].as_str().or_else(|| msg["payload"]["text"].as_str()).unwrap_or("the turn did not complete").to_string())
                    }
                }
                "turn.completed" => complete = true,
                "turn.failed" => failure = Some(msg["error"]["message"].as_str().unwrap_or("Codex turn failed").to_string()),
                "error" => failure = Some(msg["message"].as_str().unwrap_or("Codex error").to_string()),
                _ => {}
            }
        }
        let status = child.wait().await.map_err(|e| e.to_string())?;
        if !complete || !status.success() {
            return Err(failure.unwrap_or_else(|| format!("{name} ended without completing the turn ({status}); see logs/{who}.log")));
        }
        if text.trim().is_empty() {
            return Err(format!("{name} returned an empty answer"));
        }
        Ok(text.trim().to_string())
    };
    let result = tokio::time::timeout(Duration::from_secs(if chat { 1800 } else { 600 }), work).await;
    kill_group(pid, libc::SIGKILL);
    let _ = child.wait().await;
    file.iter().for_each(|f| drop(std::fs::remove_file(f)));
    result.map_err(|_| format!("{name} timed out"))?
}

#[cfg(test)]
#[path = "../checks/codex.rs"]
mod tests;
