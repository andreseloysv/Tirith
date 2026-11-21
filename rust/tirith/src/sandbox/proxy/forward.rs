//! Passing the request on and the bytes both ways until either side closes.
use super::request::Request;
use tokio::{io::AsyncWriteExt, net::TcpStream};

pub async fn forward(
    req: &Request<'_>,
    path: &str,
    c: &mut TcpStream,
    up: &mut TcpStream,
    rest: &[u8],
) -> Result<(), String> {
    if req.connect() {
        c.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
            .await
            .map_err(|e| e.to_string())?;
    } else {
        up.write_all(upstream_head(req, path).as_bytes())
            .await
            .map_err(|e| e.to_string())?;
    }
    up.write_all(rest).await.map_err(|e| e.to_string())?;
    let _ = tokio::io::copy_bidirectional(c, up).await;
    Ok(())
}

/// The plain-HTTP request as the destination gets it: the path only, no proxy or connection headers.
fn upstream_head(req: &Request, path: &str) -> String {
    let mut out = format!("{} {path} {}\r\n", req.method, req.version);
    req.headers
        .iter()
        .filter(|h| {
            let name = h
                .split(':')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            !matches!(name.as_str(), "connection" | "keep-alive") && !name.starts_with("proxy-")
        })
        .for_each(|h| out += &format!("{h}\r\n"));
    out += "Connection: close\r\n\r\n";
    out
}
