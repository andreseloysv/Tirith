//! `tirith net tunnel <host> <port>`: stdin/stdout through the proxy (ssh's ProxyCommand inside the sandbox).
use base64::Engine;
use std::{
    io::{Read, Write},
    net::TcpStream,
};

pub fn tunnel(host: &str, port: &str) -> Result<(), String> {
    let s = open(host, port)?;
    let mut up = s.try_clone().map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let _ = std::io::copy(&mut std::io::stdin(), &mut up);
        let _ = up.shutdown(std::net::Shutdown::Write);
    });
    pump(s)
}

/// The proxy's address and credentials, from HTTPS_PROXY.
fn proxy() -> Result<(String, String), String> {
    let url = std::env::var("HTTPS_PROXY")
        .map_err(|_| "no HTTPS_PROXY: the tunnel works inside OpenBot's sandbox")?;
    let rest = url
        .strip_prefix("http://")
        .ok_or("HTTPS_PROXY is not an http:// proxy")?
        .trim_end_matches('/');
    let (cred, addr) = rest
        .rsplit_once('@')
        .ok_or("HTTPS_PROXY has no credentials")?;
    Ok((cred.to_string(), addr.to_string()))
}

/// A connection through the proxy to `host:port`, once the proxy said yes.
fn open(host: &str, port: &str) -> Result<TcpStream, String> {
    let (cred, addr) = proxy()?;
    let mut s = TcpStream::connect(&addr).map_err(|e| format!("proxy {addr}: {e}"))?;
    let auth = base64::engine::general_purpose::STANDARD.encode(cred);
    write!(s, "CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\nProxy-Authorization: Basic {auth}\r\n\r\n").map_err(|e| e.to_string())?;
    let (mut head, mut b) = (Vec::new(), [0u8; 1]);
    while !head.ends_with(b"\r\n\r\n")
        && head.len() < 16384
        && s.read(&mut b).map_err(|e| e.to_string())? == 1
    {
        head.push(b[0]);
    }
    let head = String::from_utf8_lossy(&head);
    (!(!head.starts_with("HTTP/1.1 200") && !head.starts_with("HTTP/1.0 200")))
        .then_some(())
        .ok_or_else(|| {
            format!(
                "proxy refused {host}:{port}: {}",
                head.lines().next().unwrap_or("no answer")
            )
        })?;
    Ok(s)
}

/// Copies what the destination sends to stdout until it closes.
fn pump(mut s: TcpStream) -> Result<(), String> {
    let (mut buf, mut out) = ([0u8; 16384], std::io::stdout());
    loop {
        match s.read(&mut buf) {
            Ok(0) | Err(_) => return Ok(()),
            Ok(n) => out
                .write_all(&buf[..n])
                .and_then(|_| out.flush())
                .map_err(|e| e.to_string())?,
        }
    }
}
