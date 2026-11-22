//! `tirith hooks install`: the agent's command hooks become HTTP hooks on the daemon.
use crate::config;
use serde_json::{json, Value};

/// Rewrites the hooks of the configured agents whose settings file uses `tirith hook …` into HTTP hooks on the daemon (a backup of each file is kept next to it). Agents without HTTP hooks keep the command bridge.
pub(super) fn install_hooks() -> Result<(), String> {
    let cfg = config::Config::load_at(&config::tirith_home());
    cfg.factory
        .command("anthropic")
        .ok_or("no [agents.anthropic] in config.toml")?;
    let path = config::expand(
        cfg.factory
            .agent::<String>("anthropic", "settings")
            .as_deref()
            .ok_or("[agents.anthropic] has no settings path")?,
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| "{}".into());
    std::fs::write(path.with_extension("json.before-tirith-http"), &text)
        .map_err(|e| e.to_string())?;
    let mut settings: Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    for event in ["PreToolUse", "PostToolUse", "UserPromptSubmit", "Stop"] {
        let ours = json!({"type": "http", "url": format!("http://127.0.0.1:{}/hook/{event}", cfg.port),
            "headers": {"X-Tirith-Lane": "$TIRITH_LANE", "X-Tirith-Role": "$TIRITH_ROLE"},
            "allowedEnvVars": ["TIRITH_LANE", "TIRITH_ROLE"], "timeout": 5});
        let mut kept = crate::setup::others(&settings["hooks"][event]);
        kept.push(json!({"hooks": [ours]}));
        settings["hooks"][event] = json!(kept);
    }
    let out = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, out + "\n").map_err(|e| e.to_string())?;
    println!(
        "{}: hooks now call http://127.0.0.1:{}/hook/<event>",
        path.display(),
        cfg.port
    );
    Ok(())
}
