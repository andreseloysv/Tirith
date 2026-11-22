// The gateway checks' fakes: an MCP server on stdio (a shell script) and one on HTTP (answering JSON and an event stream), no model anywhere.
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// A stdio MCP server listing read_mail, send_mail, delete_mail and label. It writes the MAIL_TOKEN it was started with to `<seen>/env` and each tools/call it got to `<seen>/calls`, out of the agent's sight.
const STDIO: &str = r#"#!/bin/sh
seen="$1"
printf '%s' "$MAIL_TOKEN" > "$seen/env"
while IFS= read -r line; do
  id=$(printf '%s' "$line" | grep -o '"id":[0-9]*' | head -1 | cut -d: -f2)
  case "$line" in
    *'"method":"initialize"'*)
      printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":"2025-03-26","capabilities":{"tools":{}},"serverInfo":{"name":"fake-mail","version":"1"}}}\n' "$id" ;;
    *'"method":"tools/list"'*)
      printf '{"jsonrpc":"2.0","id":%s,"result":{"tools":[{"name":"read_mail","description":"reads","inputSchema":{"type":"object"}},{"name":"send_mail","description":"sends","inputSchema":{"type":"object"}},{"name":"delete_mail","description":"deletes","inputSchema":{"type":"object"}},{"name":"label","description":"labels","inputSchema":{"type":"object"}}]}}\n' "$id" ;;
    *'"method":"tools/call"'*)
      printf '%s\n' "$line" >> "$seen/calls"
      printf '{"jsonrpc":"2.0","id":%s,"result":{"content":[{"type":"text","text":"done"}]}}\n' "$id" ;;
  esac
done
"#;

/// Writes the stdio server into `dir`: its argv for `command`.
pub fn stdio_server(dir: &Path, seen: &Path) -> Vec<String> {
    let script = dir.join("fake-mail.sh");
    std::fs::write(&script, STDIO).unwrap();
    [
        "/bin/sh".into(),
        script.display().to_string(),
        seen.display().to_string(),
    ]
    .to_vec()
}

/// What the HTTP server was sent: each request's headers (lowercase name, value) and its body.
pub type Hits = Arc<Mutex<Vec<(Vec<(String, String)>, Value)>>>;

async fn answer(State(hits): State<Hits>, headers: HeaderMap, body: String) -> Response {
    let msg: Value = serde_json::from_str(&body).unwrap_or_default();
    let seen = headers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()));
    hits.lock().unwrap().push((seen.collect(), msg.clone()));
    let id = msg["id"].clone();
    let reply = |result: Value| json!({"jsonrpc": "2.0", "id": id, "result": result});
    match msg["method"].as_str().unwrap_or("") {
        "initialize" => {
            let r = reply(
                json!({"protocolVersion": "2025-03-26", "capabilities": {"tools": {}}, "serverInfo": {"name": "fake-web", "version": "1"}}),
            );
            ([("mcp-session-id", "web-session-1")], axum::Json(r)).into_response()
        }
        "tools/list" => {
            let tools = json!({"tools": [{"name": "fetch", "description": "fetches", "inputSchema": {"type": "object"}}, {"name": "post", "description": "posts", "inputSchema": {"type": "object"}}]});
            let stream = format!("event: message\ndata: {}\n\n", reply(tools));
            ([("content-type", "text/event-stream")], stream).into_response()
        }
        "tools/call" => axum::Json(reply(
            json!({"content": [{"type": "text", "text": "fetched"}]}),
        ))
        .into_response(),
        _ => axum::http::StatusCode::ACCEPTED.into_response(),
    }
}

/// The HTTP MCP server on a free port: its port and what it was sent.
pub async fn http_server() -> (u16, Hits) {
    let hits: Hits = Default::default();
    let router = axum::Router::new()
        .route("/mcp", post(answer))
        .with_state(hits.clone());
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { drop(axum::serve(listener, router).await) });
    (port, hits)
}
