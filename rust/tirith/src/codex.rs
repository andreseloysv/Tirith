//! Codex's local catalog and JSONL turns. Conversations resume native Codex threads; text jobs run read-only.
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

pub async fn turn(app: &Shared, role: &str, project: &str, resume: Option<&str>, prompt: (&str, &str), mut event: impl FnMut(&Value), started: impl FnOnce(u32)) -> Result<String, String> {
    let (instructions, input) = prompt;
    let setting = models::role(app, role);
    let cfg = app.cfg();
    let agent = cfg.agents.get("codex").ok_or("Codex is not configured")?;
    let mut argv = vec![agent.command.clone(), "exec".into(), "--json".into(), "--skip-git-repo-check".into()];
    if role == "manager" {
        argv.push("--dangerously-bypass-approvals-and-sandbox".into());
    } else {
        argv.extend(["--sandbox".into(), "read-only".into(), "--ephemeral".into()]);
    }
    if let Some(m) = setting.model {
        argv.extend(["--model".into(), m]);
    }
    options(&cfg, &mut argv, setting.effort.as_deref());
    argv.extend(["-c".into(), format!("developer_instructions={}", toml::Value::String(instructions.into()))]);
    if let Some(id) = resume {
        argv.extend(["resume".into(), id.into()]);
    }
    argv.push("-".into());
    let (mut child, mut stdin, stdout) = manager::spawn_chat(app, role, project, argv)?;
    let pid = child.id().unwrap_or(0);
    started(pid);
    let work = async {
        stdin.write_all(input.as_bytes()).await.map_err(|e| e.to_string())?;
        drop(stdin);
        let mut lines = BufReader::new(stdout).lines();
        let (mut text, mut complete, mut failure) = (String::new(), false, None);
        while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
            let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
            event(&msg);
            match msg["type"].as_str().unwrap_or("") {
                "item.completed" if msg["item"]["type"] == "agent_message" => {
                    text.push_str(msg["item"]["text"].as_str().unwrap_or(""));
                    text.push('\n');
                }
                "turn.completed" => complete = true,
                "turn.failed" => failure = Some(msg["error"]["message"].as_str().unwrap_or("Codex turn failed").to_string()),
                "error" => failure = Some(msg["message"].as_str().unwrap_or("Codex error").to_string()),
                _ => {}
            }
        }
        let status = child.wait().await.map_err(|e| e.to_string())?;
        if !complete || !status.success() {
            return Err(failure.unwrap_or_else(|| format!("Codex ended without completing the turn ({status}); see logs/{role}.log")));
        }
        if text.trim().is_empty() {
            return Err("Codex returned an empty answer".into());
        }
        Ok(text.trim().to_string())
    };
    let result = tokio::time::timeout(Duration::from_secs(if role == "manager" { 1800 } else { 600 }), work).await;
    kill_group(pid, libc::SIGKILL);
    let _ = child.wait().await;
    result.map_err(|_| "Codex turn timed out".to_string())?
}
