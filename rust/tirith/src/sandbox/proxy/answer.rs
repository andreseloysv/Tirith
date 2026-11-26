//! The proxy's own answers: errors and refusals.
use super::record::record;
use crate::app::Shared;
use tokio::{io::AsyncWriteExt, net::TcpStream};

pub async fn answer(
    c: &mut TcpStream,
    status: &str,
    extra: &str,
    body: &str,
) -> Result<(), String> {
    let text = format!("HTTP/1.1 {status}\r\n{extra}Content-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    c.write_all(text.as_bytes())
        .await
        .map_err(|e| e.to_string())
}

/// Records the refusal and tells the caller which rule blocks the destination.
pub async fn refuse(
    app: &Shared,
    c: &mut TcpStream,
    who: &str,
    host: &str,
    port: u16,
    rule: &str,
) -> Result<(), String> {
    let body = format!("Blocked by NeoBot: {host} (rule {rule}). The owner blocked it; it is allowed again with: neobot net unblock {rule}\n");
    refuse_with(app, c, (who, host, port), (rule, &body)).await
}

/// The same refusal with its own words (`body`): a destination off an allowlist.
pub async fn refuse_with(
    app: &Shared,
    c: &mut TcpStream,
    (who, host, port): (&str, &str, u16),
    (rule, body): (&str, &str),
) -> Result<(), String> {
    record(app, who, host, port, Some(rule));
    answer(
        c,
        "403 Forbidden",
        &format!("X-Tirith-Blocked: {rule}\r\n"),
        body,
    )
    .await
}
