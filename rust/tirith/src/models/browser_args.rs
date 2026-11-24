//! The browser MCP every agent run carries, in its own dialect.
use crate::app::Shared;
use serde_json::{json, Value};
const PROXY: [&str; 4] = ["HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY", "NO_PROXY"];
/// The browser MCP (Playwright, `tools/browser/launch.mjs` of this repo; its node_modules stay in `<home>/browser-mcp`) every agent run carries in its own dialect, so no agent's settings file is touched: Muse one `-c mcpServers.tirith_browser=…` at the argv's end (never the whole map; its env takes only the whole-value `${TIRITH_HOME}` both spawns set), Codex and anthropic right after the program (Codex `-c mcp_servers.tirith_browser={…}` plus desktop-only `node_repl` off, no `~/.codex` profile; anthropic `--mcp-config=`/`--allowedTools=` for `mcp__tirith_browser__*`, `=` so its variadic flags swallow nothing later). Lanes always get `--isolated` (several at once, nothing left behind, never the owner's browser); managers, bots and the CEO get NeoBot's persistent profile (`<home>/browser/profile`, the launcher's default), `--isolated` instead while another live session holds that profile, or `--extension` (the owner's own Edge) with `[browser] use = "mine"` in config.toml. No node or no launcher: no browser, and the run goes on without one. Agents spawn MCP servers with a clean env (only the variables named in this config), so inside NeoBot's sandbox the server env also names `TIRITH_SANDBOXED` (1) and the four proxy variables, forwarded by name and resolved by each agent from its own env, where `wrap` put them (Muse and anthropic as a whole-value `${VAR}`, Codex through `env_vars`): without them the launcher never learns it runs sandboxed (Chromium starts its own sandbox, macOS refuses to nest it — "sandbox init: Operation not permitted" — and Edge dies) nor which proxy to leave through. Muse accepts no literal env value, so its flag rides as `${TIRITH_SANDBOXED}`; the launcher still sees plain "1". A browser that cannot start ends no run: Codex's servers are optional by default, Muse's are required unless their `mode` is `optional` (a `required` key is not read there, and beside `mode` it drops every server).
pub fn browser_args_for(app: &Shared, alias: &str, lane: bool) -> Vec<String> {
    let Some((node, args)) = super::browser::browser_server(app, lane) else {
        return Vec::new();
    };
    let home = app.home.display().to_string();
    let boxed = crate::sandbox::sandboxed(app);
    match alias {
        "muse" => vec![
            "-c".into(),
            format!(
                "mcpServers.tirith_browser={}",
                json!({"command": node, "args": args, "mode": "optional", "env": env(boxed, "${TIRITH_HOME}", "${TIRITH_SANDBOXED}")})
            ),
        ],
        "anthropic" => vec![
            format!(
                "--mcp-config={}",
                json!({"mcpServers": {"tirith_browser": {"command": node, "args": args, "env": env(boxed, &home, "1")}}})
            ),
            "--allowedTools=mcp__tirith_browser__*".into(),
        ],
        "codex" => codex_args(&node, &args, &home, boxed),
        _ => Vec::new(),
    }
}
/// A TOML string literal.
fn t(s: &str) -> String {
    toml::Value::String(s.into()).to_string()
}
/// The server's env for Muse and anthropic: `TIRITH_HOME`, and inside the sandbox the proxy variables by name and `TIRITH_SANDBOXED`.
fn env(boxed: bool, home: &str, sandboxed: &str) -> Value {
    let extra = if boxed {
        PROXY
            .iter()
            .map(|k| (k.to_string(), json!(format!("${{{k}}}"))))
            .chain([("TIRITH_SANDBOXED".to_string(), json!(sandboxed))])
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    Value::Object(
        [("TIRITH_HOME".to_string(), json!(home))]
            .into_iter()
            .chain(extra)
            .collect(),
    )
}
/// Codex's `-c mcp_servers.tirith_browser={…}` (the proxy through `env_vars`) and its desktop-only `node_repl` off, as a whole server: Codex refuses to load («invalid transport») a switched-off server with no command, which is what `.enabled=false` alone makes where the desktop app never wrote one.
fn codex_args(node: &str, args: &[String], home: &str, boxed: bool) -> Vec<String> {
    let list = args.iter().map(|a| t(a)).collect::<Vec<_>>().join(", ");
    let named = if boxed {
        format!(
            ", env_vars = [{}]",
            PROXY.iter().map(|k| t(k)).collect::<Vec<_>>().join(", ")
        )
    } else {
        String::new()
    };
    let flag = if boxed {
        ", TIRITH_SANDBOXED = \"1\""
    } else {
        ""
    };
    Vec::from([
        "-c".to_string(),
        format!("mcp_servers.tirith_browser={{command = {}, args = [{}], startup_timeout_sec = 30, tool_timeout_sec = 90, env = {{TIRITH_HOME = {}{}}}{}}}", t(node), list, t(home), flag, named),
        "-c".into(),
        "mcp_servers.node_repl={command=\"node_repl\",enabled=false}".into(),
    ])
}
