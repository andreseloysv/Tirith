//! The CLI side: one HTTP/1.1 request to the daemon on 127.0.0.1 per command.

use serde_json::Value;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// `flag("--name")`: the value after a `--name` argument.
pub type Flag<'a> = &'a dyn Fn(&str) -> Option<String>;

/// One request to the daemon; the body on 2xx, the daemon's message otherwise.
pub fn call(method: &str, path: &str, body: Option<Value>) -> Result<String, String> {
    call_with(method, path, body.map(|b| b.to_string()).unwrap_or_default(), &[], Duration::from_secs(30))
}

pub fn call_with(method: &str, path: &str, body: String, headers: &[(&str, String)], timeout: Duration) -> Result<String, String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], crate::config::Config::load().port));
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).map_err(|_| "tirith is not running: start it with `tirith serve`".to_string())?;
    s.set_read_timeout(Some(timeout)).ok();
    let extra: String = headers.iter().map(|(k, v)| format!("{k}: {v}\r\n")).collect();
    let req = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n{body}", body.len());
    s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).map_err(|e| e.to_string())?;
    let raw = String::from_utf8_lossy(&raw);
    let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
    let code: u16 = head.split_whitespace().nth(1).and_then(|c| c.parse().ok()).unwrap_or(0);
    (200..300).contains(&code).then(|| body.to_string()).ok_or_else(|| body.trim().to_string())
}

/// Prints a lane-shaped answer as one line, anything else as "ok".
pub fn print(r: Result<String, String>) -> Result<(), String> {
    let body = r?;
    match serde_json::from_str::<Value>(&body) {
        Ok(v) if v.get("id").is_some() => println!("{} {} — {}", v["id"].as_str().unwrap_or(""), v["status"].as_str().unwrap_or(""), v["item"].as_str().unwrap_or("")),
        _ => println!("ok"),
    }
    Ok(())
}
