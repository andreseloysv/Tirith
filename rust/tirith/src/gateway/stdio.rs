//! The gateway's side facing the agent: MCP over stdio, one JSON-RPC message per line. It answers `initialize` and `ping` itself and hands `tools/list` and every `tools/call` to the daemon, which decides; it holds no secret and no connector.
use super::{door::Door, rpc};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

/// Serves the agent on `input` and `out` as bot `bot` until the agent closes its side; Err, before anything is served, when the daemon does not prove this run is that bot.
pub fn serve(
    input: impl BufRead,
    mut out: impl Write,
    door: &Door,
    bot: &str,
) -> Result<(), String> {
    let body = json!({"bot": bot, "pid": std::process::id()});
    let opened = door.ask("POST", "/api/mcp/open", &body)?;
    let session = opened["session"]
        .as_str()
        .ok_or("the daemon opened no gateway")?;
    for line in input.lines() {
        let line = line.map_err(|e| e.to_string())?;
        let reply = match serde_json::from_str::<Value>(&line) {
            _ if line.trim().is_empty() => None,
            Ok(msg) => answer(door, session, &msg),
            Err(_) => Some(error(Value::Null, -32700, "not JSON")),
        };
        if let Some(reply) = reply {
            writeln!(out, "{reply}")
                .and_then(|_| out.flush())
                .map_err(|e| e.to_string())?;
        }
    }
    drop(door.ask("DELETE", &format!("/api/mcp/{session}"), &Value::Null));
    Ok(())
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

/// The answer to one message; none to a notification or to an answer.
fn answer(door: &Door, session: &str, msg: &Value) -> Option<Value> {
    let (id, method) = (msg.get("id")?.clone(), msg.get("method")?.as_str()?);
    let params = &msg["params"];
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": params["protocolVersion"].as_str().unwrap_or(rpc::VERSION),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "tirith-connectors", "version": crate::bundle::VERSION},
        })),
        "ping" => Ok(json!({})),
        "tools/list" => door
            .ask("GET", &format!("/api/mcp/{session}/tools"), &Value::Null)
            .map(|v| json!({"tools": v["tools"]})),
        "tools/call" => {
            let body = json!({"name": params["name"], "arguments": params["arguments"]});
            door.ask("POST", &format!("/api/mcp/{session}/call"), &body)
        }
        _ => {
            return Some(error(
                id,
                -32601,
                &format!("tirith mcp has no method {method}"),
            ))
        }
    };
    Some(match result {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err(why) => error(id, -32603, &why),
    })
}
