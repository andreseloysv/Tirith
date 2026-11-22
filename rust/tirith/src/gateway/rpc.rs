//! JSON-RPC 2.0 as MCP speaks it, both ways: a request, a notification, an HTTP answer's messages, an answer read, and a tool result that says it failed.
use serde_json::{json, Value};

pub fn request(id: u64, method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

pub fn notification(method: &str) -> Value {
    json!({"jsonrpc": "2.0", "method": method})
}

/// The messages of an HTTP answer of type `kind`: JSON, or an event stream's `data:` lines.
pub fn messages(kind: &str, body: &str) -> impl Iterator<Item = Value> {
    let msgs: Vec<Value> = if kind.starts_with("text/event-stream") {
        let data = body.lines().filter_map(|l| l.strip_prefix("data:"));
        data.filter_map(|d| serde_json::from_str(d.trim()).ok())
            .collect()
    } else {
        serde_json::from_str(body).into_iter().collect()
    };
    msgs.into_iter()
}

/// Whether `msg` answers request `id`.
pub fn answers(msg: &Value, id: u64) -> bool {
    msg["id"].as_u64() == Some(id) && (msg.get("result").is_some() || msg.get("error").is_some())
}

/// The result an answer carries, or its error's message.
pub fn answer(msg: Value) -> Result<Value, String> {
    match msg.get("error") {
        Some(e) => Err(e["message"].as_str().unwrap_or("error").to_string()),
        None => Ok(msg["result"].clone()),
    }
}

/// A tool call's result that failed: the agent reads `text`.
pub fn failed(text: &str) -> Value {
    json!({"content": [{"type": "text", "text": text}], "isError": true})
}

/// The protocol version a gateway says when the agent names none.
pub const VERSION: &str = "2025-03-26";

/// What `initialize` sends a connector.
pub fn hello() -> Value {
    json!({"protocolVersion": VERSION, "capabilities": {}, "clientInfo": {"name": "tirith", "version": crate::bundle::VERSION}})
}
