//! The connector gateway a bot's run carries, in its agent's own dialect (as `browser_args.rs` does the browser): only a `bot-<id>` role granted a connector gets one, never a lane or a manager.
use crate::app::Shared;
use serde_json::json;

/// `tirith mcp --bot <id>` as the MCP server `tirith_connectors`. Its env names only `TIRITH_HOME` and the run's own `TIRITH_TOKEN`, forwarded by name (Muse and anthropic as a whole-value `${VAR}`, Codex through `env_vars`): no secret is in it, nor in its arguments. Muse's is optional, like the browser's; anthropic's tools are allowed by `--allowedTools=mcp__tirith_connectors__*`, the gateway refusing what the verdict refuses.
pub fn gateway_args_for(app: &Shared, alias: &str, role: &str) -> Vec<String> {
    let Some(bot) = role.strip_prefix("bot-") else {
        return vec![];
    };
    let exe = crate::app::exe().map(|e| e.display().to_string());
    let Some(exe) = exe.filter(|_| !app.cfg().factory.granted(role).is_empty()) else {
        return vec![];
    };
    let args = ["mcp", "--bot", bot];
    let home = app.home.display().to_string();
    match alias {
        "muse" => vec![
            "-c".into(),
            format!(
                "mcpServers.tirith_connectors={}",
                json!({"command": exe, "args": args, "mode": "optional", "env": {"TIRITH_HOME": "${TIRITH_HOME}", "TIRITH_TOKEN": "${TIRITH_TOKEN}"}})
            ),
        ],
        "anthropic" => vec![
            format!(
                "--mcp-config={}",
                json!({"mcpServers": {"tirith_connectors": {"command": exe, "args": args, "env": {"TIRITH_HOME": home, "TIRITH_TOKEN": "${TIRITH_TOKEN}"}}}})
            ),
            "--allowedTools=mcp__tirith_connectors__*".into(),
        ],
        "codex" => {
            let t = |s: &str| toml::Value::String(s.into()).to_string();
            let list: Vec<String> = args.iter().map(|a| t(a)).collect();
            vec![
                "-c".into(),
                format!("mcp_servers.tirith_connectors={{command = {}, args = [{}], startup_timeout_sec = 60, tool_timeout_sec = 300, env = {{TIRITH_HOME = {}}}, env_vars = [\"TIRITH_TOKEN\"]}}", t(&exe), list.join(", "), t(&home)),
            ]
        }
        _ => vec![],
    }
}

/// Adds the gateway where the browser's flags go: Muse's at the argv's end, the others' right after the program.
pub(super) fn add_gateway(app: &Shared, argv: &mut Vec<String>, alias: &str, role: &str) {
    let at = if alias == "muse" { argv.len() } else { 1 };
    drop(argv.splice(at..at, gateway_args_for(app, alias, role)));
}
